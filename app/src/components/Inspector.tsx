import { useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { open } from '@tauri-apps/plugin-dialog'
import { Copy, Download, GitCompare, Plus, Save, Trash2, X } from 'lucide-react'
import { api, formatBytes, type Asset, type Project, type Version } from '../lib/api'
import { toastError, useToast } from '../state/toast'
import { StatusBadge } from './ui'
import { CodeView, DiffView } from './CodeView'

/** Panel lateral: metadatos editables, versiones, código y comparación. */
export function Inspector({ asset, projects, onClose, onChanged }: { asset: Asset | null; projects: Project[]; onClose: () => void; onChanged: () => void }) {
  return (
    <AnimatePresence>
      {asset && (
        <>
          <motion.div className="fixed inset-0 z-30 bg-black/40" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onClick={onClose} />
          <motion.aside key={asset.id} className="fixed top-0 right-0 z-30 flex h-full w-[640px] flex-col border-l border-white/[0.06] bg-surface shadow-2xl"
            initial={{ x: 660 }} animate={{ x: 0 }} exit={{ x: 660 }} transition={{ type: 'spring', damping: 30, stiffness: 300 }}>
            <Body asset={asset} projects={projects} onClose={onClose} onChanged={onChanged} />
          </motion.aside>
        </>
      )}
    </AnimatePresence>
  )
}

function Body({ asset, projects, onClose, onChanged }: { asset: Asset; projects: Project[]; onClose: () => void; onChanged: () => void }) {
  const push = useToast((s) => s.push)
  const [name, setName] = useState(asset.name)
  const [projectId, setProjectId] = useState(asset.project_id ?? '')
  const [meta, setMeta] = useState<[string, string][]>(Object.entries(asset.metadata).map(([k, v]) => [k, typeof v === 'string' ? v : JSON.stringify(v)]))
  const [versions, setVersions] = useState<Version[]>([])
  const [shown, setShown] = useState(asset.version)
  const [text, setText] = useState<string | null>(null)
  const [cmp, setCmp] = useState<{ a: number; b: number; ta: string; tb: string } | null>(null)

  useEffect(() => { api.versions(asset.id).then(setVersions).catch(toastError) }, [asset.id, asset.version])
  useEffect(() => { api.text(asset.id, shown).then(setText).catch(toastError) }, [asset.id, shown])

  async function save() {
    try {
      const metadata = Object.fromEntries(meta.filter(([k]) => k.trim()).map(([k, v]) => [k.trim(), v]))
      await api.updateAsset(asset.id, { name, projectId, metadata })
      push('ok', 'Cambios guardados'); onChanged()
    } catch (e) { toastError(e) }
  }
  async function addVersion() {
    try {
      const f = await open({ multiple: false }); if (!f || Array.isArray(f)) return
      const r = await api.addVersion(asset.id, f)
      push(r.new_version ? 'ok' : 'info', r.new_version ? `Nueva versión v${r.asset.version}` : 'El archivo es idéntico a la versión actual'); onChanged()
    } catch (e) { toastError(e) }
  }
  async function restore(v: number) {
    try { const d = await open({ directory: true }); if (!d || Array.isArray(d)) return; push('ok', `Restaurado en ${await api.restore(asset.id, d, v)}`) } catch (e) { toastError(e) }
  }
  async function compare(a: number, b: number) {
    try {
      const [ta, tb] = await Promise.all([api.text(asset.id, a), api.text(asset.id, b)])
      if (ta == null || tb == null) return push('info', 'Solo se pueden comparar archivos de texto')
      setCmp({ a, b, ta, tb })
    } catch (e) { toastError(e) }
  }

  return (
    <>
      <div className="flex items-start justify-between border-b border-white/[0.06] p-5">
        <div><div className="flex items-center gap-2"><h2 className="text-lg font-semibold text-white">{asset.name}</h2><StatusBadge s={asset.status} /></div>
          <p className="mt-1 text-xs text-zinc-500">{asset.original_name} · {asset.format.toUpperCase()} · {formatBytes(asset.size)} · v{asset.version}</p></div>
        <button onClick={onClose} className="text-zinc-500 hover:text-white"><X size={18} /></button>
      </div>
      <div className="flex-1 space-y-6 overflow-y-auto p-5">
        <section className="grid grid-cols-2 gap-3">
          <label className="text-xs text-zinc-400">Nombre<input className="input mt-1 w-full" value={name} onChange={(e) => setName(e.target.value)} /></label>
          <label className="text-xs text-zinc-400">Proyecto<select className="input mt-1 w-full" value={projectId} onChange={(e) => setProjectId(e.target.value)}>
            <option value="">Global (compartido)</option>{projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}</select></label>
        </section>
        <section>
          <div className="mb-2 flex items-center justify-between"><h3 className="text-sm font-medium text-zinc-300">Metadatos</h3>
            <button className="text-xs text-accent hover:underline" onClick={() => setMeta([...meta, ['', '']])}><Plus size={12} className="inline" /> Campo</button></div>
          <div className="space-y-2">{meta.map(([k, v], i) => (
            <div key={i} className="flex gap-2">
              <input className="input w-40" placeholder="clave" value={k} onChange={(e) => setMeta(meta.map((m, j) => j === i ? [e.target.value, m[1]] : m))} />
              <input className="input flex-1" placeholder="valor" value={v} onChange={(e) => setMeta(meta.map((m, j) => j === i ? [m[0], e.target.value] : m))} />
              <button className="text-zinc-500 hover:text-red-400" onClick={() => setMeta(meta.filter((_, j) => j !== i))}><Trash2 size={14} /></button>
            </div>))}
            {meta.length === 0 && <p className="text-xs text-zinc-500">Sin metadatos.</p>}</div>
          <button className="btn-primary mt-3" onClick={save}><Save size={14} />Guardar</button>
        </section>
        <section>
          <div className="mb-2 flex items-center justify-between"><h3 className="text-sm font-medium text-zinc-300">Versiones</h3>
            <button className="btn-ghost py-1 text-xs" onClick={addVersion}><Plus size={12} />Nueva versión</button></div>
          <div className="card divide-y divide-white/[0.04]">{versions.map((v, i) => (
            <div key={v.version} className={`flex items-center gap-3 px-3 py-2 text-xs ${shown === v.version ? 'bg-violet/10' : ''}`}>
              <button className="font-medium text-white hover:text-accent" onClick={() => setShown(v.version)}>v{v.version}</button>
              <span className="text-zinc-500">{new Date(v.created_at).toLocaleString()}</span>
              <span className="font-mono text-zinc-600">{v.sha256.slice(0, 10)}</span>
              <span className="ml-auto flex gap-2">
                {versions[i + 1] && <button title="Comparar con anterior" className="text-zinc-400 hover:text-accent" onClick={() => compare(versions[i + 1].version, v.version)}><GitCompare size={14} /></button>}
                <button title="Recuperar esta versión" className="text-zinc-400 hover:text-accent" onClick={() => restore(v.version)}><Download size={14} /></button>
              </span>
            </div>))}</div>
        </section>
        {cmp && <section><div className="mb-2 flex justify-between"><h3 className="text-sm font-medium text-zinc-300">v{cmp.a} → v{cmp.b}</h3>
          <button className="text-xs text-zinc-500 hover:text-white" onClick={() => setCmp(null)}>Cerrar</button></div><DiffView a={cmp.ta} b={cmp.tb} /></section>}
        {text != null && <section><div className="mb-2 flex justify-between"><h3 className="text-sm font-medium text-zinc-300">Contenido v{shown}</h3>
          <button className="text-xs text-accent hover:underline" onClick={() => navigator.clipboard.writeText(text).then(() => push('ok', 'Código copiado'))}><Copy size={12} className="inline" /> Copiar</button></div>
          <CodeView code={text} /></section>}
      </div>
    </>
  )
}
