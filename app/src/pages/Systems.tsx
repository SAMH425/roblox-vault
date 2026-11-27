import { useCallback, useEffect, useState } from 'react'
import { motion } from 'motion/react'
import { open } from '@tauri-apps/plugin-dialog'
import { AlertTriangle, Download, PackagePlus, Plus, Puzzle, Save, Search, Trash2 } from 'lucide-react'
import { api, STATUSES, type Asset, type Project, type Status, type SystemInput, type VaultSystem } from '../lib/api'
import { toastError, useToast } from '../state/toast'
import { Empty, Modal, PageHeader, StatusBadge } from '../components/ui'

const EMPTY: SystemInput = { name: '', project_id: null, description: '', install: '', config: '', compatibility: '', tests: '' }
const FIELDS: [keyof SystemInput, string][] = [['description', 'Descripción'], ['install', 'Instalación'], ['config', 'Configuración'], ['compatibility', 'Compatibilidad'], ['tests', 'Pruebas']]

export default function Systems() {
  const [items, setItems] = useState<VaultSystem[] | null>(null)
  const [projects, setProjects] = useState<Project[]>([])
  const [assets, setAssets] = useState<Asset[]>([])
  const [q, setQ] = useState('')
  const [selId, setSelId] = useState<string | null>(null)
  const [creating, setCreating] = useState(false)
  const [newName, setNewName] = useState('')
  const push = useToast((s) => s.push)

  const load = useCallback(() => api.systems({ query: q || undefined }).then(setItems).catch((e) => { setItems([]); toastError(e) }), [q])
  useEffect(() => { const t = setTimeout(load, 200); return () => clearTimeout(t) }, [load])
  useEffect(() => { api.projects().then(setProjects).catch(() => {}); api.assets().then(setAssets).catch(() => {}) }, [])
  const sel = items?.find((s) => s.id === selId) ?? null

  async function create() {
    try { const s = await api.createSystem({ ...EMPTY, name: newName }); setCreating(false); setNewName(''); await load(); setSelId(s.id); push('ok', 'Sistema creado') } catch (e) { toastError(e) }
  }

  return (
    <div>
      <PageHeader title="Systems Library" subtitle="Grupos de recursos que funcionan juntos" actions={<><button className="btn-ghost" onClick={async () => {
        try { const r = await api.installTemplates(); push('ok', `Plantillas BASE: ${r.systems.length} sistemas (${r.systems.reduce((n, s) => n + s.added, 0)} archivos nuevos, ${r.systems.reduce((n, s) => n + s.updated, 0)} actualizados)`); load() } catch (e) { toastError(e) }
      }}><PackagePlus size={16} />Instalar plantillas BASE</button><button className="btn-primary" onClick={() => setCreating(true)}><Plus size={16} />Nuevo sistema</button></>} />
      <div className="grid grid-cols-[300px_1fr] gap-5">
        <div>
          <div className="relative mb-3"><Search size={16} className="absolute top-2.5 left-3 text-zinc-500" /><input className="input w-full pl-9" placeholder="Buscar sistemas…" value={q} onChange={(e) => setQ(e.target.value)} /></div>
          {items === null ? <div className="skeleton h-40" /> : items.length === 0 ? <p className="text-sm text-zinc-500">Sin sistemas.</p> :
            <div className="space-y-2">{items.map((s) => (
              <motion.button key={s.id} layout onClick={() => setSelId(s.id)} className={`card w-full p-3 text-left transition-colors ${selId === s.id ? 'border-violet/50 bg-violet/10' : 'hover:border-white/15'}`}>
                <div className="flex items-center justify-between"><span className="font-medium text-white">{s.name}</span><StatusBadge s={s.status} /></div>
                <div className="mt-1 flex gap-3 text-xs text-zinc-500"><span>{s.members.length} recursos</span>{s.warnings.length > 0 && <span className="text-amber-300">{s.warnings.length} avisos</span>}</div>
              </motion.button>))}</div>}
        </div>
        {sel ? <Detail key={sel.id + sel.updated_at + sel.members.length} sys={sel} projects={projects} assets={assets} reload={load} />
          : <Empty icon={<Puzzle size={22} />} title="Elige o crea un sistema" text="Ejemplo: INVENTORY SYSTEM con InventoryController, InventoryData, ItemConfig, InventoryUI y sus RemoteEvents." />}
      </div>
      <Modal open={creating} onClose={() => setCreating(false)} title="Nuevo sistema">
        <input className="input w-full" placeholder="INVENTORY SYSTEM" value={newName} onChange={(e) => setNewName(e.target.value)} autoFocus />
        <div className="mt-4 flex justify-end gap-2"><button className="btn-ghost" onClick={() => setCreating(false)}>Cancelar</button><button className="btn-primary" disabled={!newName.trim()} onClick={create}>Crear</button></div>
      </Modal>
    </div>
  )
}

function Detail({ sys, projects, assets, reload }: { sys: VaultSystem; projects: Project[]; assets: Asset[]; reload: () => void }) {
  const [f, setF] = useState<SystemInput>({ name: sys.name, project_id: sys.project_id, description: sys.description, install: sys.install, config: sys.config, compatibility: sys.compatibility, tests: sys.tests })
  const [addId, setAddId] = useState('')
  const [role, setRole] = useState('')
  const push = useToast((s) => s.push)
  const available = assets.filter((a) => !sys.members.some((m) => m.asset.id === a.id))

  const run = (p: Promise<unknown>, msg?: string) => p.then(() => { if (msg) push('ok', msg); reload() }).catch(toastError)
  async function restore() {
    try { const d = await open({ directory: true }); if (!d || Array.isArray(d)) return; const r = await api.restoreSystem(sys.id, d); push('ok', `${r.length} archivo(s) recuperados`) } catch (e) { toastError(e) }
  }

  return (
    <div className="card space-y-5 p-5">
      <div className="flex items-center gap-3">
        <input className="input flex-1 text-base font-medium" value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} />
        <select className="input" value={f.project_id ?? ''} onChange={(e) => setF({ ...f, project_id: e.target.value || null })}>
          <option value="">Global</option>{projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}</select>
        <select className="input" value={sys.status} onChange={(e) => run(api.setSystemStatus(sys.id, e.target.value as Status))}>{STATUSES.map((s) => <option key={s}>{s}</option>)}</select>
      </div>
      {sys.warnings.length > 0 && <div className="rounded-xl border border-amber-400/30 bg-amber-400/5 p-3 text-sm text-amber-200">
        {sys.warnings.map((w) => <div key={w} className="flex items-center gap-2"><AlertTriangle size={14} />{w}</div>)}</div>}
      <section>
        <h3 className="mb-2 text-sm font-medium text-zinc-300">Recursos</h3>
        <div className="divide-y divide-white/[0.04] rounded-xl border border-white/[0.06]">
          {sys.members.map((m) => (
            <div key={m.asset.id} className="flex items-center gap-3 px-3 py-2 text-sm">
              <span className="w-28 truncate text-xs text-accent">{m.role || '—'}</span><span className="flex-1 text-white">{m.asset.name}</span>
              <span className="text-xs text-zinc-500 uppercase">{m.asset.format} · v{m.asset.version}</span><StatusBadge s={m.asset.status} />
              <button className="text-zinc-500 hover:text-red-400" onClick={() => run(api.removeMember(sys.id, m.asset.id), 'Recurso quitado')}><Trash2 size={14} /></button>
            </div>))}
          {sys.members.length === 0 && <p className="p-3 text-xs text-zinc-500">Añade recursos de la biblioteca.</p>}
        </div>
        <div className="mt-2 flex gap-2">
          <select className="input flex-1" value={addId} onChange={(e) => setAddId(e.target.value)}><option value="">Añadir recurso…</option>
            {available.map((a) => <option key={a.id} value={a.id}>{a.name} ({a.format})</option>)}</select>
          <input className="input w-40" placeholder="rol (ej. controller)" value={role} onChange={(e) => setRole(e.target.value)} />
          <button className="btn-ghost" disabled={!addId} onClick={() => { run(api.addMember(sys.id, addId, role), 'Recurso añadido'); setAddId(''); setRole('') }}><Plus size={14} /></button>
        </div>
      </section>
      <section className="grid grid-cols-2 gap-3">
        {FIELDS.map(([k, label]) => (
          <label key={k} className={`text-xs text-zinc-400 ${k === 'description' ? 'col-span-2' : ''}`}>{label}
            <textarea className="input mt-1 h-24 w-full resize-y font-mono text-xs" value={(f[k] as string) ?? ''} onChange={(e) => setF({ ...f, [k]: e.target.value })} /></label>))}
      </section>
      <div className="flex gap-2">
        <button className="btn-primary" onClick={() => run(api.updateSystem(sys.id, f), 'Sistema guardado')}><Save size={14} />Guardar</button>
        <button className="btn-ghost" onClick={restore}><Download size={14} />Recuperar sistema completo</button>
      </div>
    </div>
  )
}
