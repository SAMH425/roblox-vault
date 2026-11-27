import { useEffect, useMemo, useState } from 'react'
import { motion } from 'motion/react'
import { AlertTriangle, ArrowDownRight, ArrowUpLeft, Lightbulb, Network, Plus, Search, Trash2 } from 'lucide-react'
import { api, type Asset, type DepReport } from '../lib/api'
import { toastError, useToast } from '../state/toast'
import { Empty, PageHeader, StatusBadge } from '../components/ui'

export default function Dependencies() {
  const [assets, setAssets] = useState<Asset[]>([])
  const [q, setQ] = useState('')
  const [sel, setSel] = useState<string | null>(null)
  const [rep, setRep] = useState<DepReport | null>(null)
  const [addId, setAddId] = useState('')
  const push = useToast((s) => s.push)

  useEffect(() => { api.assets().then(setAssets).catch(toastError) }, [])
  const load = (id = sel) => { if (id) api.deps(id).then(setRep).catch(toastError) }
  useEffect(() => { setRep(null); load() }, [sel])

  const list = useMemo(() => assets.filter((a) => a.name.toLowerCase().includes(q.toLowerCase())), [assets, q])
  const current = assets.find((a) => a.id === sel)
  const run = (p: Promise<unknown>, msg: string) => p.then(() => { push('ok', msg); load() }).catch(toastError)

  return (
    <div>
      <PageHeader title="Dependencies" subtitle="Qué necesita cada recurso para funcionar" />
      <div className="grid grid-cols-[280px_1fr] gap-5">
        <div>
          <div className="relative mb-3"><Search size={16} className="absolute top-2.5 left-3 text-zinc-500" /><input className="input w-full pl-9" placeholder="Buscar recurso…" value={q} onChange={(e) => setQ(e.target.value)} /></div>
          <div className="max-h-[70vh] space-y-1 overflow-y-auto">{list.map((a) => (
            <button key={a.id} onClick={() => setSel(a.id)} className={`flex w-full items-center justify-between rounded-xl px-3 py-2 text-left text-sm ${sel === a.id ? 'bg-violet/15 text-white ring-1 ring-violet/30' : 'text-zinc-300 hover:bg-panel/60'}`}>
              <span className="truncate">{a.name}</span><span className="text-[10px] text-zinc-500 uppercase">{a.kind}</span></button>))}
            {list.length === 0 && <p className="text-sm text-zinc-500">Sin recursos.</p>}</div>
        </div>
        {!current ? <Empty icon={<Network size={22} />} title="Elige un recurso" text="Verás de qué depende (directo y en cadena), quién depende de él y los require() de su código sin resolver." />
          : !rep ? <div className="skeleton h-64" /> : (
            <motion.div key={current.id} initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="space-y-4">
              <div className="card flex items-center justify-between p-4"><div><h2 className="text-lg font-semibold text-white">{current.name}</h2><p className="text-xs text-zinc-500">{current.original_name}</p></div><StatusBadge s={current.status} /></div>
              {rep.problems.length > 0 && <div className="rounded-xl border border-red-500/30 bg-red-500/5 p-3 text-sm text-red-300">{rep.problems.map((p) => <div key={p} className="flex items-center gap-2"><AlertTriangle size={14} />{p}</div>)}</div>}
              {(rep.suggestions.length > 0 || rep.unresolved_requires.length > 0) && (
                <div className="card p-4">
                  <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-zinc-300"><Lightbulb size={14} className="text-accent" />Detectado en el código (require)</h3>
                  {rep.suggestions.map((a) => <div key={a.id} className="flex items-center justify-between py-1 text-sm"><span className="text-white">{a.name}</span>
                    <button className="btn-ghost py-1 text-xs" onClick={() => run(api.addDep(current.id, a.id, 'require'), `Dependencia ${a.name} añadida`)}><Plus size={12} />Añadir</button></div>)}
                  {rep.unresolved_requires.map((n) => <div key={n} className="py-1 text-sm text-amber-300">{n} — no está en la bóveda</div>)}
                </div>)}
              <div className="grid grid-cols-2 gap-4">
                <div className="card p-4">
                  <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-zinc-300"><ArrowDownRight size={14} />Depende de ({rep.all.length})</h3>
                  {rep.all.length === 0 && <p className="text-xs text-zinc-500">Ninguna.</p>}
                  {rep.all.map((d) => (
                    <div key={d.asset.id} className="flex items-center gap-2 py-1 text-sm" style={{ paddingLeft: (d.depth - 1) * 14 }}>
                      <span className="flex-1 truncate text-white">{d.asset.name}</span><StatusBadge s={d.asset.status} />
                      {d.depth === 1 && <button className="text-zinc-500 hover:text-red-400" onClick={() => run(api.removeDep(current.id, d.asset.id), 'Dependencia quitada')}><Trash2 size={13} /></button>}
                    </div>))}
                  <div className="mt-3 flex gap-2">
                    <select className="input flex-1 text-xs" value={addId} onChange={(e) => setAddId(e.target.value)}><option value="">Añadir dependencia…</option>
                      {assets.filter((a) => a.id !== current.id && !rep.direct.some((d) => d.asset.id === a.id)).map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}</select>
                    <button className="btn-ghost" disabled={!addId} onClick={() => { run(api.addDep(current.id, addId), 'Dependencia añadida'); setAddId('') }}><Plus size={14} /></button>
                  </div>
                </div>
                <div className="card p-4">
                  <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-zinc-300"><ArrowUpLeft size={14} />Lo usan ({rep.dependents.length})</h3>
                  {rep.dependents.length === 0 && <p className="text-xs text-zinc-500">Nadie.</p>}
                  {rep.dependents.map((a) => <button key={a.id} onClick={() => setSel(a.id)} className="flex w-full items-center gap-2 py-1 text-left text-sm text-white hover:text-accent"><span className="flex-1 truncate">{a.name}</span><StatusBadge s={a.status} /></button>)}
                </div>
              </div>
            </motion.div>)}
      </div>
    </div>
  )
}
