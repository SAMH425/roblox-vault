import { useCallback, useEffect, useState } from 'react'
import { motion } from 'motion/react'
import { open } from '@tauri-apps/plugin-dialog'
import { Download, FileUp, Search, Boxes, LayoutGrid, List } from 'lucide-react'
import { api, formatBytes, STATUSES, type Asset, type Project, type Status } from '../lib/api'
import { toastError, useToast } from '../state/toast'
import { Empty, PageHeader, StatusBadge } from '../components/ui'
import { Inspector } from '../components/Inspector'
import { Thumb } from '../components/Thumb'

type Sort = 'relevance' | 'recent' | 'name' | 'size' | 'status'

/** Biblioteca genérica: se usa para modelos, scripts, etc. filtrando por `kind`. */
export default function Library({ kind, title, subtitle, exts }: { kind?: string; title: string; subtitle: string; exts?: string[] }) {
  const [items, setItems] = useState<Asset[] | null>(null)
  const [projects, setProjects] = useState<Project[]>([])
  const [q, setQ] = useState('')
  const [projectId, setProjectId] = useState('')
  const [status, setStatusF] = useState('')
  const [sort, setSort] = useState<Sort>('relevance')
  const [view, setView] = useState<'list' | 'grid'>(kind === 'model' ? 'grid' : 'list')
  const [sel, setSel] = useState<Asset | null>(null)
  const [thumbBump, setThumbBump] = useState(0)
  const push = useToast((s) => s.push)

  const load = useCallback(() => {
    const p = q.trim() ? api.ranked(q, { kind, projectId: projectId || undefined }).then((r) => r.map((x) => x.asset)) : api.assets({ kind, projectId: projectId || undefined })
    p.then(setItems).catch((e) => { setItems([]); toastError(e) })
  }, [kind, projectId, q])
  useEffect(() => { const t = setTimeout(load, 200); return () => clearTimeout(t) }, [load])
  useEffect(() => { api.projects().then(setProjects).catch(() => {}) }, [])

  const shown = (items ?? []).filter((a) => !status || a.status === status).sort((a, b) =>
    sort === 'relevance' ? 0 : sort === 'name' ? a.name.localeCompare(b.name) : sort === 'size' ? b.size - a.size : sort === 'status' ? a.status.localeCompare(b.status) : b.updated_at.localeCompare(a.updated_at))

  function changed() { load(); if (sel) api.assets({ kind }).then((l) => setSel(l.find((x) => x.id === sel.id) ?? null)).catch(() => {}) }

  async function importFiles() {
    try {
      const sel = await open({ multiple: true, filters: exts ? [{ name: title, extensions: exts }] : undefined })
      if (!sel) return
      const paths = Array.isArray(sel) ? sel : [sel]
      const res = await api.importFiles(paths, projectId || undefined)
      const dup = res.filter((r) => r.duplicate_of).length
      push('ok', `${res.length - dup} importado(s)${dup ? `, ${dup} duplicado(s) ya existían` : ''}. Originales intactos.`)
      load()
    } catch (e) { toastError(e) }
  }

  async function restore(a: Asset) {
    try {
      const dir = await open({ directory: true })
      if (!dir || Array.isArray(dir)) return
      push('ok', `Restaurado en ${await api.restore(a.id, dir)}`)
    } catch (e) { toastError(e) }
  }

  async function changeStatus(a: Asset, s: Status) {
    try { await api.setStatus(a.id, s); load() } catch (e) { toastError(e) }
  }

  return (
    <div>
      <PageHeader title={title} subtitle={subtitle} actions={<button className="btn-primary" onClick={importFiles}><FileUp size={16} />Importar</button>} />
      <div className="mb-4 flex gap-3">
        <div className="relative flex-1"><Search size={16} className="absolute top-2.5 left-3 text-zinc-500" />
          <input className="input w-full pl-9" placeholder="Buscar por nombre o contenido…" value={q} onChange={(e) => setQ(e.target.value)} /></div>
        <select className="input" value={projectId} onChange={(e) => setProjectId(e.target.value)}>
          <option value="">Todos los proyectos</option>{projects.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
        </select>
        <select className="input" value={status} onChange={(e) => setStatusF(e.target.value)}><option value="">Todos los estados</option>{STATUSES.map((s) => <option key={s}>{s}</option>)}</select>
        <select className="input" value={sort} onChange={(e) => setSort(e.target.value as Sort)}>
          <option value="relevance">Relevancia</option><option value="recent">Recientes</option><option value="name">Nombre</option><option value="size">Tamaño</option><option value="status">Estado</option></select>
        <div className="flex rounded-xl border border-white/[0.08] p-0.5">
          {(['list', 'grid'] as const).map((v) => <button key={v} onClick={() => setView(v)} className={`rounded-lg px-2.5 ${view === v ? 'bg-violet/20 text-white' : 'text-zinc-500'}`}>{v === 'list' ? <List size={15} /> : <LayoutGrid size={15} />}</button>)}
        </div>
      </div>

      {items === null ? <div className="space-y-2">{[0, 1, 2].map((i) => <div key={i} className="skeleton h-14" />)}</div>
        : shown.length === 0 ? <Empty icon={<Boxes size={22} />} title="Nada por aquí" text="Importa archivos: se copian a la bóveda con SHA-256 y el original no se toca." action={<button className="btn-ghost" onClick={importFiles}><FileUp size={16} />Importar</button>} />
        : view === 'grid' ? (
          <div className="grid grid-cols-4 gap-4">{shown.map((a, i) => (
            <motion.button key={a.id} onClick={() => setSel(a)} className="card p-4 text-left transition-colors hover:border-violet/40" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.02 }} whileHover={{ y: -2 }}>
              <div className="mb-3 grid h-32 place-items-center overflow-hidden rounded-xl bg-panel/60"><Thumb asset={a} bump={thumbBump} /></div>
              <div className="truncate font-medium text-white">{a.name}</div>
              <div className="mt-1 flex items-center justify-between text-xs text-zinc-500"><span>v{a.version} · {formatBytes(a.size)}</span><StatusBadge s={a.status} /></div>
            </motion.button>))}</div>
        ) : (
          <div className="card overflow-hidden">
            <table className="w-full text-sm">
              <thead className="text-left text-xs text-zinc-500"><tr className="border-b border-white/[0.06]">
                <th className="px-4 py-3">Nombre</th><th>Formato</th><th>Proyecto</th><th>Versión</th><th>Tamaño</th><th>Estado</th><th /></tr></thead>
              <tbody>
                {shown.map((a, i) => (
                  <motion.tr key={a.id} onClick={() => setSel(a)} initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ delay: i * 0.015 }} className="cursor-pointer border-b border-white/[0.04] hover:bg-panel/40">
                    <td className="px-4 py-3"><div className="font-medium text-white">{a.name}</div><div className="font-mono text-[11px] text-zinc-500" title={a.sha256}>{a.sha256.slice(0, 16)}…</div></td>
                    <td className="text-zinc-400"><span className="uppercase">{a.format}</span>{typeof a.metadata.script_type === 'string' && <span className="ml-2 text-xs text-accent">{a.metadata.script_type}</span>}</td>
                    <td className="text-zinc-400">{a.project_name ?? 'Global'}</td>
                    <td className="text-zinc-400">v{a.version}</td>
                    <td className="text-zinc-400">{formatBytes(a.size)}</td>
                    <td>
                      <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}><StatusBadge s={a.status} />
                        <select className="bg-transparent text-xs text-zinc-500" value={a.status} onChange={(e) => changeStatus(a, e.target.value as Status)}>
                          {STATUSES.map((s) => <option key={s}>{s}</option>)}</select></div>
                    </td>
                    <td className="pr-4 text-right"><button className="btn-ghost py-1.5" onClick={(e) => { e.stopPropagation(); restore(a) }}><Download size={14} />Recuperar</button></td>
                  </motion.tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      <Inspector asset={sel} projects={projects} onClose={() => setSel(null)} onChanged={changed} onThumb={() => setThumbBump((b) => b + 1)} />
    </div>
  )
}
