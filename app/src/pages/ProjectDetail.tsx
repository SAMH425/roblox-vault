import { useEffect, useState } from 'react'
import { ArrowLeft, Bug, GitCommit, Landmark, Lightbulb, Save, Trash2 } from 'lucide-react'
import { api, type NoteKind, type ProjectContext } from '../lib/api'
import { toastError, useToast } from '../state/toast'
import { StatusBadge } from '../components/ui'

const KINDS: { k: NoteKind; label: string; icon: typeof Bug }[] = [
  { k: 'decision', label: 'Decisiones técnicas', icon: Lightbulb }, { k: 'bug', label: 'Errores conocidos', icon: Bug },
  { k: 'change', label: 'Cambios recientes', icon: GitCommit }, { k: 'architecture', label: 'Notas de arquitectura', icon: Landmark },
]

export default function ProjectDetail({ id, onBack }: { id: string; onBack: () => void }) {
  const [ctx, setCtx] = useState<ProjectContext | null>(null)
  const [f, setF] = useState({ name: '', description: '', architecture: '' })
  const [draft, setDraft] = useState<Record<NoteKind, string>>({ decision: '', bug: '', change: '', architecture: '' })
  const push = useToast((s) => s.push)
  const load = () => api.projectContext(id).then((c) => { setCtx(c); setF({ name: c.project.name, description: c.project.description, architecture: c.project.architecture }) }).catch(toastError)
  useEffect(() => { load() }, [id])
  if (!ctx) return <div className="skeleton h-64" />

  async function save() { try { await api.updateProject(id, f.name, f.description, f.architecture); push('ok', 'Proyecto guardado'); load() } catch (e) { toastError(e) } }
  async function addNote(k: NoteKind) { try { await api.addNote(id, k, draft[k]); setDraft({ ...draft, [k]: '' }); load() } catch (e) { toastError(e) } }

  return (
    <div className="space-y-5">
      <button className="flex items-center gap-1 text-sm text-zinc-400 hover:text-white" onClick={onBack}><ArrowLeft size={14} />Proyectos</button>
      <div className="card space-y-3 p-5">
        <input className="input w-full text-lg font-semibold" value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} />
        <textarea className="input h-20 w-full resize-y" placeholder="Descripción" value={f.description} onChange={(e) => setF({ ...f, description: e.target.value })} />
        <textarea className="input h-32 w-full resize-y font-mono text-xs" placeholder="Arquitectura (servicios, carpetas, flujo cliente/servidor…)" value={f.architecture} onChange={(e) => setF({ ...f, architecture: e.target.value })} />
        <button className="btn-primary" onClick={save}><Save size={14} />Guardar</button>
      </div>
      <div className="grid grid-cols-2 gap-4">
        {KINDS.map(({ k, label, icon: Icon }) => (
          <div key={k} className="card p-4">
            <h3 className="mb-2 flex items-center gap-2 text-sm font-medium text-zinc-300"><Icon size={14} className="text-accent" />{label}</h3>
            <div className="max-h-48 space-y-1.5 overflow-y-auto">{ctx.notes.filter((n) => n.kind === k).map((n) => (
              <div key={n.id} className="group flex gap-2 rounded-lg bg-bg/50 px-2.5 py-1.5 text-sm"><span className="flex-1 whitespace-pre-wrap text-zinc-200">{n.text}</span>
                <span className="text-[10px] text-zinc-600">{new Date(n.created_at).toLocaleDateString()}</span>
                <button className="text-zinc-600 opacity-0 group-hover:opacity-100 hover:text-red-400" onClick={() => api.deleteNote(n.id).then(load).catch(toastError)}><Trash2 size={12} /></button></div>))}</div>
            <div className="mt-2 flex gap-2"><input className="input flex-1 text-xs" placeholder="Añadir…" value={draft[k]} onChange={(e) => setDraft({ ...draft, [k]: e.target.value })} onKeyDown={(e) => e.key === 'Enter' && draft[k].trim() && addNote(k)} /></div>
          </div>))}
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="card p-4"><h3 className="mb-2 text-sm font-medium text-zinc-300">Recursos ({ctx.assets.length})</h3>
          {ctx.assets.map((a) => <div key={a.id} className="flex items-center justify-between py-1 text-sm"><span className="text-white">{a.name} <span className="text-xs text-zinc-500 uppercase">{a.format} · v{a.version}</span></span><StatusBadge s={a.status} /></div>)}
          {ctx.assets.length === 0 && <p className="text-xs text-zinc-500">Asigna recursos a este proyecto desde su inspector.</p>}</div>
        <div className="card p-4"><h3 className="mb-2 text-sm font-medium text-zinc-300">Sistemas ({ctx.systems.length})</h3>
          {ctx.systems.map((s) => <div key={s.id} className="flex items-center justify-between py-1 text-sm"><span className="text-white">{s.name}</span><StatusBadge s={s.status} /></div>)}
          <h3 className="mt-4 mb-2 text-sm font-medium text-zinc-300">Actividad reciente</h3>
          {ctx.recent.slice(0, 8).map((a) => <div key={a.id} className="flex justify-between text-xs"><span className="text-accent">{a.action}</span><span className="text-zinc-500">{a.target}</span></div>)}</div>
      </div>
    </div>
  )
}
