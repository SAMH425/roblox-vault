import { useEffect, useState } from 'react'
import { motion } from 'motion/react'
import { Activity as ActIcon, Boxes, FolderGit2, HardDrive } from 'lucide-react'
import { api, formatBytes, type Activity, type Stats } from '../lib/api'
import { toastError } from '../state/toast'
import { PageHeader } from '../components/ui'

export default function Dashboard() {
  const [stats, setStats] = useState<Stats | null>(null)
  const [acts, setActs] = useState<Activity[]>([])
  const [pending, setPending] = useState(0)
  useEffect(() => { api.stats().then(setStats).catch(toastError); api.activity(8).then(setActs).catch(() => {}); api.captures('pending').then((c) => setPending(c.length)).catch(() => {}) }, [])

  const cards = stats ? [
    { label: 'Recursos', value: stats.assets, icon: Boxes },
    { label: 'Proyectos', value: stats.projects, icon: FolderGit2 },
    { label: 'Almacenamiento', value: formatBytes(stats.storage_bytes), icon: HardDrive },
    { label: 'Actividad 24 h', value: stats.activity_24h, icon: ActIcon },
  ] : []

  return (
    <div>
      <PageHeader title="Dashboard" subtitle="Estado real de tu bóveda" />
      {pending > 0 && <div className="mb-4 rounded-xl border border-amber-400/30 bg-amber-400/5 px-4 py-3 text-sm text-amber-200">{pending} recurso(s) de Claude esperan tu aprobación en Activity → Captura.</div>}
      <div className="grid grid-cols-4 gap-4">
        {stats ? cards.map((c, i) => (
          <motion.div key={c.label} className="card p-5" initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.05 }}>
            <div className="flex items-center justify-between text-zinc-400"><span className="text-sm">{c.label}</span><c.icon size={18} className="text-accent" /></div>
            <div className="mt-3 text-3xl font-semibold text-white">{c.value}</div>
          </motion.div>
        )) : [0, 1, 2, 3].map((i) => <div key={i} className="skeleton h-28" />)}
      </div>

      <div className="mt-6 grid grid-cols-3 gap-4">
        <div className="card p-5">
          <h3 className="mb-3 text-sm font-medium text-zinc-300">Por tipo</h3>
          {stats?.by_kind.length ? stats.by_kind.map(([k, n]) => <Bar key={k} label={k} n={n} max={stats.assets} />) : <p className="text-sm text-zinc-500">Sin recursos todavía.</p>}
        </div>
        <div className="card p-5">
          <h3 className="mb-3 text-sm font-medium text-zinc-300">Por validación</h3>
          {stats?.by_status.length ? stats.by_status.map(([k, n]) => <Bar key={k} label={k} n={n} max={stats.assets} />) : <p className="text-sm text-zinc-500">Sin recursos todavía.</p>}
        </div>
        <div className="card p-5">
          <h3 className="mb-3 text-sm font-medium text-zinc-300">Actividad reciente</h3>
          {acts.length ? acts.map((a) => (
            <div key={a.id} className="flex justify-between border-b border-white/[0.04] py-1.5 text-xs last:border-0">
              <span className="text-accent">{a.action}</span><span className="max-w-[55%] truncate text-zinc-400">{a.target}</span>
            </div>
          )) : <p className="text-sm text-zinc-500">Sin actividad.</p>}
        </div>
      </div>
    </div>
  )
}

function Bar({ label, n, max }: { label: string; n: number; max: number }) {
  return (
    <div className="mb-2.5">
      <div className="mb-1 flex justify-between text-xs"><span className="text-zinc-300">{label}</span><span className="text-zinc-500">{n}</span></div>
      <div className="h-1.5 overflow-hidden rounded-full bg-panel"><motion.div className="h-full bg-violet" initial={{ width: 0 }} animate={{ width: `${(n / Math.max(max, 1)) * 100}%` }} /></div>
    </div>
  )
}
