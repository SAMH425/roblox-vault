import { useEffect, useState } from 'react'
import { motion } from 'motion/react'
import { Clapperboard, Repeat } from 'lucide-react'
import { api, type Asset } from '../lib/api'
import { toastError } from '../state/toast'
import { Empty, PageHeader, StatusBadge } from '../components/ui'
import Library from './Library'

interface Anim { path: string; keyframes: number; length_s: number; looped: boolean; poses: string[] }

/** Animaciones = KeyframeSequence exportadas como .rbxm/.rbxmx. */
export default function Animations() {
  const [items, setItems] = useState<Asset[] | null>(null)
  const [all, setAll] = useState(false)
  useEffect(() => { api.assets({ kind: 'animation' }).then(setItems).catch((e) => { setItems([]); toastError(e) }) }, [all])

  if (all) return <div><button className="btn-ghost mb-4" onClick={() => setAll(false)}>← Resumen</button><Library kind="animation" title="Animations" subtitle="Importa KeyframeSequence (.rbxm / .rbxmx)" exts={['rbxm', 'rbxmx']} /></div>
  return (
    <div>
      <PageHeader title="Animations" subtitle="KeyframeSequence guardadas en .rbxm / .rbxmx" actions={<button className="btn-primary" onClick={() => setAll(true)}><Clapperboard size={16} />Importar y gestionar</button>} />
      {items === null ? <div className="skeleton h-40" /> : items.length === 0
        ? <Empty icon={<Clapperboard size={22} />} title="Sin animaciones" text="En Studio: clic derecho en la KeyframeSequence → Save to File. Al importarla se detectan keyframes, duración, bucle y articulaciones." />
        : <div className="grid grid-cols-3 gap-4">{items.map((a, i) => {
            const anims = (a.metadata.animations as Anim[] | undefined) ?? []
            return (
              <motion.div key={a.id} className="card p-4" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.03 }}>
                <div className="flex items-center justify-between"><span className="font-medium text-white">{a.name}</span><StatusBadge s={a.status} /></div>
                {anims.map((an) => (
                  <div key={an.path} className="mt-3 text-xs text-zinc-400">
                    <div className="flex gap-3"><span>{an.keyframes} keyframes</span><span>{an.length_s.toFixed(2)} s</span>{an.looped && <span className="flex items-center gap-1 text-accent"><Repeat size={11} />bucle</span>}</div>
                    {an.poses.length > 0 && <div className="mt-2 flex flex-wrap gap-1">{an.poses.slice(0, 12).map((p) => <span key={p} className="rounded bg-panel px-1.5 py-0.5 text-[10px]">{p}</span>)}{an.poses.length > 12 && <span>+{an.poses.length - 12}</span>}</div>}
                  </div>))}
              </motion.div>)
          })}</div>}
      <p className="mt-4 text-xs text-zinc-500">La reproducción animada sobre un rig llegará con el plugin de Studio (Fase 6); aquí se guardan, versionan y recuperan los archivos originales.</p>
    </div>
  )
}
