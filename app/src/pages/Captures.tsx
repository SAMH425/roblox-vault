import { useEffect, useState } from 'react'
import { Check, Inbox, X } from 'lucide-react'
import { api, type Capture } from '../lib/api'
import { toastError, useToast } from '../state/toast'
import { Empty } from '../components/ui'

/** Cola de captura híbrida: lo que Claude registró sin pruebas espera tu aprobación. */
export function Captures({ onChange }: { onChange?: () => void }) {
  const [items, setItems] = useState<Capture[] | null>(null)
  const [state, setState] = useState('pending')
  const push = useToast((s) => s.push)
  const load = () => api.captures(state || undefined).then(setItems).catch((e) => { setItems([]); toastError(e) })
  useEffect(() => { load() }, [state])

  async function decide(c: Capture, ok: boolean) {
    try { await api.decideCapture(c.id, ok); push('ok', ok ? 'Aprobado y guardado en la bóveda' : 'Descartado'); load(); onChange?.() } catch (e) { toastError(e) }
  }
  return (
    <div>
      <div className="mb-3 flex gap-1">{[['pending', 'Pendientes'], ['approved', 'Aprobadas'], ['rejected', 'Rechazadas'], ['', 'Todas']].map(([k, l]) => (
        <button key={k} onClick={() => setState(k)} className={`rounded-lg px-3 py-1.5 text-xs ${state === k ? 'bg-violet/20 text-white' : 'text-zinc-400 hover:text-white'}`}>{l}</button>))}</div>
      {items === null ? <div className="skeleton h-24" /> : items.length === 0
        ? <Empty icon={<Inbox size={22} />} title="Nada en la cola" text="Cuando Claude registre algo sin pruebas, aparecerá aquí para que lo apruebes." />
        : <div className="card divide-y divide-white/[0.04]">{items.map((c) => (
            <div key={c.id} className="flex items-start gap-4 p-4 text-sm">
              <div className="flex-1">
                <div className="font-mono text-xs text-white">{c.source_path}</div>
                <div className="mt-1 flex gap-3 text-xs text-zinc-500"><span>{c.actor}</span><span>{new Date(c.created_at).toLocaleString()}</span>
                  <span className={c.tested ? (c.test_passed ? 'text-ok' : 'text-red-400') : 'text-amber-300'}>{c.tested ? (c.test_passed ? 'pruebas OK' : 'pruebas fallaron') : 'sin probar'}</span></div>
                {c.note && <p className="mt-1 text-xs text-zinc-300">{c.note}</p>}
                {c.test_output && <pre className="mt-2 max-h-24 overflow-auto rounded-lg bg-bg/70 p-2 text-[11px] text-zinc-400">{c.test_output}</pre>}
              </div>
              {c.state === 'pending' ? <div className="flex gap-2">
                <button className="btn-primary py-1.5" onClick={() => decide(c, true)}><Check size={14} />Aprobar</button>
                <button className="btn-ghost py-1.5" onClick={() => decide(c, false)}><X size={14} />Rechazar</button></div>
                : <span className={`text-xs ${c.state === 'approved' ? 'text-ok' : 'text-zinc-500'}`}>{c.state === 'approved' ? 'aprobada' : 'rechazada'}</span>}
            </div>))}</div>}
    </div>
  )
}
