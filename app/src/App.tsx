import { useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { Activity as ActIcon, Boxes, FileText, Music, Image as ImageIcon, Code2, FolderGit2, LayoutDashboard, Network, Puzzle, Settings as Cog, Clapperboard } from 'lucide-react'
import Dashboard from './pages/Dashboard'
import Library from './pages/Library'
import Projects from './pages/Projects'
import Systems from './pages/Systems'
import Dependencies from './pages/Dependencies'
import Animations from './pages/Animations'
import { UpdateBanner, useUpdater } from './components/Updater'
import { useThumbWorker } from './components/ThumbWorker'
import { useThumbBus } from './state/thumbs'
import { Activity, Settings } from './pages/ActivitySettings'
import { Toasts } from './components/ui'
import { isTauri } from './lib/api'

const NAV = [
  { id: 'dash', label: 'Dashboard', icon: LayoutDashboard },
  { id: 'models', label: '3D Asset Library', icon: Boxes },
  { id: 'code', label: 'Code Vault', icon: Code2 },
  { id: 'icons', label: 'UI & Icons', icon: ImageIcon },
  { id: 'audio', label: 'Audio', icon: Music },
  { id: 'docs', label: 'Docs', icon: FileText },
  { id: 'systems', label: 'Systems Library', icon: Puzzle },
  { id: 'projects', label: 'Project Memory', icon: FolderGit2 },
  { id: 'anim', label: 'Animations', icon: Clapperboard },
  { id: 'deps', label: 'Dependencies', icon: Network },
  { id: 'activity', label: 'Activity', icon: ActIcon },
  { id: 'settings', label: 'Settings', icon: Cog },
] as const
type Page = (typeof NAV)[number]['id']

function render(p: Page, u: ReturnType<typeof useUpdater>) {
  switch (p) {
    case 'dash': return <Dashboard />
    case 'models': return <Library kind="model" title="3D Asset Library" subtitle="GLB, GLTF, FBX, OBJ, RBXM, RBXMX" exts={['glb', 'gltf', 'fbx', 'obj', 'rbxm', 'rbxmx']} />
    case 'code': return <Library kind="script" title="Code Vault" subtitle="Scripts y ModuleScripts Luau" exts={['lua', 'luau']} />
    case 'icons': return <Library kind="texture" title="UI & Icons" subtitle="Iconos e imágenes de interfaz (PNG, JPG, TGA)" exts={['png', 'jpg', 'jpeg', 'tga', 'bmp']} />
    case 'audio': return <Library kind="audio" title="Audio" subtitle="Música, efectos y voces (MP3, OGG, WAV)" exts={['mp3', 'ogg', 'wav']} />
    case 'docs': return <Library kind="doc" title="Docs" subtitle="Guías, planes y notas de cada juego (MD, TXT, PDF)" exts={['md', 'txt', 'pdf']} />
    case 'projects': return <Projects />
    case 'activity': return <Activity />
    case 'settings': return <Settings updater={u} />
    case 'systems': return <Systems />
    case 'anim': return <Animations />
    case 'deps': return <Dependencies />
  }
}

export default function App() {
  const [page, setPage] = useState<Page>('dash')
  const u = useUpdater()
  const bump = useThumbBus((s) => s.bump)
  const thumbs = useThumbWorker(bump)
  return (
    <div className="flex h-full">
      <aside className="flex w-60 shrink-0 flex-col border-r border-white/[0.06] bg-surface/60 p-4">
        <div className="mb-8 flex items-center gap-2.5 px-2 pt-1">
          <div className="grid h-8 w-8 place-items-center rounded-lg bg-violet text-sm font-bold text-white">V</div>
          <span className="text-sm font-semibold tracking-[0.18em] text-white">ROBLOX VAULT</span>
        </div>
        <nav className="flex flex-col gap-0.5">
          {NAV.map((n) => (
            <button key={n.id} onClick={() => setPage(n.id)} className={`relative flex items-center gap-3 rounded-xl px-3 py-2 text-sm transition-colors ${page === n.id ? 'text-white' : 'text-zinc-400 hover:text-zinc-200'}`}>
              {page === n.id && <motion.span layoutId="nav" className="absolute inset-0 rounded-xl bg-violet/15 ring-1 ring-violet/30" transition={{ type: 'spring', duration: 0.35 }} />}
              <n.icon size={17} className="relative" /><span className="relative">{n.label}</span>
            </button>
          ))}
        </nav>
        <div className="mt-auto"><UpdateBanner u={u} />
          {thumbs && <div className="mx-1 mb-3 rounded-xl bg-panel/60 p-2.5 text-[11px] text-zinc-400">Miniaturas {thumbs.done}/{thumbs.total}
            <div className="mt-1.5 h-1 overflow-hidden rounded-full bg-bg"><div className="h-full bg-violet transition-all" style={{ width: `${(thumbs.done / thumbs.total) * 100}%` }} /></div></div>}
        </div>
        <div className="px-2 text-[11px] text-zinc-500">v{u.version || '0.1.0'} · Fase 6{!isTauri() && <div className="mt-1 text-amber-300">Modo navegador: sin bóveda</div>}</div>
      </aside>
      <main className="flex-1 overflow-y-auto">
        <AnimatePresence mode="wait">
          <motion.div key={page} className="mx-auto max-w-6xl p-8" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -4 }} transition={{ duration: 0.18 }}>
            {render(page, u)}
          </motion.div>
        </AnimatePresence>
      </main>
      <Toasts />
    </div>
  )
}
