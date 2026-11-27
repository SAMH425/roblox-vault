import { useCallback, useEffect, useState } from 'react'
import { AnimatePresence, motion } from 'motion/react'
import { check, type Update } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import { getVersion } from '@tauri-apps/api/app'
import { DownloadCloud, RefreshCw } from 'lucide-react'
import { isTauri } from '../lib/api'

type State = { kind: 'idle' | 'checking' | 'none' | 'available' | 'downloading' | 'error'; msg?: string; pct?: number }

/** Busca actualizaciones publicadas en GitHub Releases, al abrir y cada 30 min. */
export function useUpdater() {
  const [st, setSt] = useState<State>({ kind: 'idle' })
  const [upd, setUpd] = useState<Update | null>(null)
  const [version, setVersion] = useState('')

  const run = useCallback(async (silent = false) => {
    if (!isTauri()) return
    if (!silent) setSt({ kind: 'checking' })
    try {
      const u = await check()
      if (u) { setUpd(u); setSt({ kind: 'available', msg: u.version }) } else setSt({ kind: 'none' })
    } catch (e) {
      const m = String(e)
      setSt({ kind: 'error', msg: /endpoint/i.test(m) ? 'Las actualizaciones solo funcionan en la versión instalada desde GitHub.' : m })
    }
  }, [])

  useEffect(() => {
    if (!isTauri()) return
    getVersion().then(setVersion).catch(() => {})
    run(true)
    const t = setInterval(() => run(true), 30 * 60 * 1000)
    return () => clearInterval(t)
  }, [run])

  async function install() {
    if (!upd) return
    let total = 0, got = 0
    setSt({ kind: 'downloading', pct: 0 })
    try {
      await upd.downloadAndInstall((ev) => {
        if (ev.event === 'Started') total = ev.data.contentLength ?? 0
        else if (ev.event === 'Progress') { got += ev.data.chunkLength; setSt({ kind: 'downloading', pct: total ? Math.round((got / total) * 100) : undefined }) }
      })
      await relaunch()
    } catch (e) { setSt({ kind: 'error', msg: String(e) }) }
  }
  return { st, version, check: () => run(false), install }
}

export function UpdateBanner({ u }: { u: ReturnType<typeof useUpdater> }) {
  const show = u.st.kind === 'available' || u.st.kind === 'downloading'
  return (
    <AnimatePresence>
      {show && (
        <motion.div initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} className="mx-1 mb-3 rounded-xl border border-violet/40 bg-violet/15 p-3 text-xs">
          <div className="flex items-center gap-2 font-medium text-white"><DownloadCloud size={14} />Versión {u.st.kind === 'available' ? u.st.msg : 'nueva'}</div>
          {u.st.kind === 'downloading'
            ? <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-panel"><div className="h-full bg-violet transition-all" style={{ width: `${u.st.pct ?? 50}%` }} /></div>
            : <button onClick={u.install} className="mt-2 flex items-center gap-1.5 rounded-lg bg-violet px-2.5 py-1 font-medium text-white hover:bg-violet/85"><RefreshCw size={12} />Actualizar y reiniciar</button>}
        </motion.div>
      )}
    </AnimatePresence>
  )
}
