import { useEffect, useState } from 'react'
import { motion } from 'motion/react'
import { FolderGit2, Plus } from 'lucide-react'
import { api, type Project } from '../lib/api'
import { toastError, useToast } from '../state/toast'
import { Empty, Modal, PageHeader } from '../components/ui'

export default function Projects() {
  const [items, setItems] = useState<Project[] | null>(null)
  const [show, setShow] = useState(false)
  const [name, setName] = useState('')
  const [desc, setDesc] = useState('')
  const push = useToast((s) => s.push)
  const load = () => api.projects().then(setItems).catch((e) => { setItems([]); toastError(e) })
  useEffect(() => { load() }, [])

  async function create() {
    try { await api.createProject(name, desc); push('ok', `Proyecto "${name}" creado`); setShow(false); setName(''); setDesc(''); load() }
    catch (e) { toastError(e) }
  }

  return (
    <div>
      <PageHeader title="Project Memory" subtitle="Cada juego tiene su propia memoria" actions={<button className="btn-primary" onClick={() => setShow(true)}><Plus size={16} />Nuevo proyecto</button>} />
      {items === null ? <div className="grid grid-cols-3 gap-4">{[0, 1, 2].map((i) => <div key={i} className="skeleton h-32" />)}</div>
        : items.length === 0 ? <Empty icon={<FolderGit2 size={22} />} title="Sin proyectos" text="Crea un proyecto por cada juego de Roblox." />
        : <div className="grid grid-cols-3 gap-4">{items.map((p, i) => (
            <motion.div key={p.id} className="card p-5 transition-colors hover:border-violet/40" initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: i * 0.04 }} whileHover={{ y: -2 }}>
              <FolderGit2 size={18} className="text-accent" />
              <h3 className="mt-3 font-medium text-white">{p.name}</h3>
              <p className="mt-1 line-clamp-2 text-sm text-zinc-400">{p.description || 'Sin descripción'}</p>
              <p className="mt-3 text-[11px] text-zinc-500">Creado {new Date(p.created_at).toLocaleDateString()}</p>
            </motion.div>))}</div>}
      <Modal open={show} onClose={() => setShow(false)} title="Nuevo proyecto">
        <div className="flex flex-col gap-3">
          <input className="input" placeholder="Nombre del juego" value={name} onChange={(e) => setName(e.target.value)} autoFocus />
          <textarea className="input h-24 resize-none" placeholder="Descripción" value={desc} onChange={(e) => setDesc(e.target.value)} />
          <div className="flex justify-end gap-2"><button className="btn-ghost" onClick={() => setShow(false)}>Cancelar</button><button className="btn-primary" disabled={!name.trim()} onClick={create}>Crear</button></div>
        </div>
      </Modal>
    </div>
  )
}
