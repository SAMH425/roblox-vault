import { AnimatePresence, motion } from 'motion/react'
import { CheckCircle2, Info, X, XCircle } from 'lucide-react'
import type { ReactNode } from 'react'
import type { Status } from '../lib/api'
import { useToast } from '../state/toast'

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  return (
    <div className="mb-6 flex items-end justify-between gap-4">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight text-white">{title}</h1>
        {subtitle && <p className="mt-1 text-sm text-zinc-400">{subtitle}</p>}
      </div>
      <div className="flex gap-2">{actions}</div>
    </div>
  )
}

const statusColor: Record<Status, string> = {
  VERIFIED: 'bg-ok/15 text-ok border-ok/30',
  'PARTIALLY VERIFIED': 'bg-accent/10 text-accent border-accent/30',
  PENDING: 'bg-amber-400/10 text-amber-300 border-amber-400/30',
  FAILED: 'bg-red-500/10 text-red-400 border-red-500/30',
  DEPRECATED: 'bg-zinc-500/10 text-zinc-400 border-zinc-500/30',
}
export const StatusBadge = ({ s }: { s: Status }) => (
  <span className={`rounded-full border px-2 py-0.5 text-[11px] font-medium ${statusColor[s]}`}>{s}</span>
)

export function Empty({ icon, title, text, action }: { icon: ReactNode; title: string; text: string; action?: ReactNode }) {
  return (
    <div className="card flex flex-col items-center justify-center gap-3 px-6 py-16 text-center">
      <div className="rounded-2xl bg-violet/10 p-3 text-accent">{icon}</div>
      <h3 className="font-medium text-white">{title}</h3>
      <p className="max-w-md text-sm text-zinc-400">{text}</p>
      {action}
    </div>
  )
}

export function Toasts() {
  const { toasts, drop } = useToast()
  const icon = { ok: <CheckCircle2 size={16} className="text-ok" />, error: <XCircle size={16} className="text-red-400" />, info: <Info size={16} className="text-accent" /> }
  return (
    <div className="pointer-events-none fixed right-5 bottom-5 z-50 flex w-96 flex-col gap-2">
      <AnimatePresence>
        {toasts.map((t) => (
          <motion.div key={t.id} layout initial={{ opacity: 0, y: 12, scale: 0.97 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, x: 40 }}
            className="card pointer-events-auto flex items-start gap-3 bg-panel/95 p-3 text-sm shadow-xl backdrop-blur">
            {icon[t.kind]}<span className="flex-1 break-words">{t.text}</span>
            <button onClick={() => drop(t.id)} className="text-zinc-500 hover:text-white"><X size={14} /></button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  )
}

export function Modal({ open, onClose, title, children }: { open: boolean; onClose: () => void; title: string; children: ReactNode }) {
  return (
    <AnimatePresence>
      {open && (
        <motion.div className="fixed inset-0 z-40 flex items-center justify-center bg-black/60 backdrop-blur-sm" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} onClick={onClose}>
          <motion.div className="card w-[440px] bg-surface p-6" initial={{ scale: 0.96, y: 8 }} animate={{ scale: 1, y: 0 }} exit={{ scale: 0.96 }} onClick={(e) => e.stopPropagation()}>
            <h2 className="mb-4 text-lg font-semibold text-white">{title}</h2>
            {children}
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}

export function PhaseNotice({ phase, what }: { phase: number; what: string }) {
  return (
    <div className="card border-dashed p-8 text-center">
      <span className="rounded-full bg-violet/15 px-3 py-1 text-xs font-medium text-accent">Fase {phase}</span>
      <p className="mt-3 text-sm text-zinc-400">{what}</p>
      <p className="mt-1 text-xs text-zinc-500">Esta sección aún no está construida. No se muestran datos ficticios.</p>
    </div>
  )
}
