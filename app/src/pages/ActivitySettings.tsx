import { useEffect, useState } from 'react'
import { Blocks, Copy, DatabaseBackup, PlugZap, RefreshCw, ShieldCheck } from 'lucide-react'
import type { useUpdater } from '../components/Updater'
import { api, type Activity as Act, type IntegrityReport } from '../lib/api'
import { toastError, useToast } from '../state/toast'
import { PageHeader } from '../components/ui'
import { Captures } from './Captures'

export function Activity() {
  const [tab, setTab] = useState<'log' | 'capture'>('log')
  const [items, setItems] = useState<Act[] | null>(null)
  useEffect(() => { api.activity(200).then(setItems).catch((e) => { setItems([]); toastError(e) }) }, [])
  return (
    <div>
      <PageHeader title="Activity" subtitle="Registro de operaciones y cola de captura" />
      <div className="mb-4 flex gap-1">{([['log', 'Registro'], ['capture', 'Captura de Claude']] as const).map(([k, l]) => (
        <button key={k} onClick={() => setTab(k)} className={`rounded-xl px-3.5 py-2 text-sm ${tab === k ? 'bg-violet/20 text-white ring-1 ring-violet/30' : 'text-zinc-400 hover:text-white'}`}>{l}</button>))}</div>
      {tab === 'capture' ? <Captures /> : <div className="card divide-y divide-white/[0.04]">
        {items === null ? <div className="skeleton m-4 h-20" /> : items.length === 0 ? <p className="p-6 text-sm text-zinc-500">Sin actividad.</p>
          : items.map((a) => (
            <div key={a.id} className="grid grid-cols-[170px_160px_1fr_auto] gap-4 px-4 py-2.5 text-sm">
              <span className="text-zinc-500">{new Date(a.at).toLocaleString()}</span>
              <span className={a.destructive ? 'text-red-400' : 'text-accent'}>{a.action}</span>
              <span className="truncate text-zinc-300">{a.target} <span className="text-zinc-500">{a.detail}</span></span>
              <span className="text-xs text-zinc-500">{a.actor}</span>
            </div>))}
      </div>}
    </div>
  )
}

export function Settings({ updater }: { updater: ReturnType<typeof useUpdater> }) {
  const [path, setPath] = useState('')
  const [rep, setRep] = useState<IntegrityReport | null>(null)
  const [busy, setBusy] = useState(false)
  const push = useToast((s) => s.push)
  const [auto, setAuto] = useState(true)
  const [bridge, setBridge] = useState<Awaited<ReturnType<typeof api.bridgeInfo>> | null>(null)
  const [confirmPlugin, setConfirmPlugin] = useState(false)
  const [mcp, setMcp] = useState<Awaited<ReturnType<typeof api.mcpInfo>> | null>(null)
  useEffect(() => { api.vaultPath().then(setPath).catch(toastError); api.mcpInfo().then(setMcp).catch(() => {}); api.bridgeInfo().then(setBridge).catch(() => {}); api.getSetting('auto_capture_tested').then((v) => setAuto(v !== 'false')).catch(() => {}) }, [])
  async function toggleAuto() { try { await api.setSetting('auto_capture_tested', String(!auto)); setAuto(!auto) } catch (e) { toastError(e) } }

  async function verify() {
    setBusy(true)
    try { setRep(await api.verify()) } catch (e) { toastError(e) } finally { setBusy(false) }
  }
  async function backup() {
    try { push('ok', `Copia creada: ${await api.backup()}`) } catch (e) { toastError(e) }
  }

  return (
    <div>
      <PageHeader title="Settings" subtitle="Almacenamiento, integridad y copias de seguridad" />
      <div className="card mb-4 p-5"><h3 className="text-sm text-zinc-400">Ubicación de la bóveda</h3><p className="mt-1 font-mono text-sm text-white">{path || '—'}</p></div>
      <div className="card mb-4 flex items-center justify-between p-5">
        <div><h3 className="font-medium text-white">Actualizaciones</h3>
          <p className="mt-1 text-sm text-zinc-400">Versión {updater.version || '—'} · {{ idle: 'Sin comprobar', checking: 'Buscando…', none: 'Estás al día', available: `Disponible ${updater.st.msg}`, downloading: 'Descargando…', error: updater.st.msg }[updater.st.kind]}</p></div>
        {updater.st.kind === 'available' ? <button className="btn-primary" onClick={updater.install}><RefreshCw size={16} />Actualizar</button>
          : <button className="btn-ghost" disabled={updater.st.kind === 'checking' || updater.st.kind === 'downloading'} onClick={updater.check}><RefreshCw size={16} />Buscar</button>}
      </div>
      <div className="card mb-4 p-5">
        <h3 className="flex items-center gap-2 font-medium text-white"><Blocks size={16} className="text-accent" />Plugin de Roblox Studio</h3>
        {!bridge ? <div className="skeleton mt-3 h-10" /> : <>
          <p className="mt-1 text-sm text-zinc-400">
            Conexión local: {bridge.running ? <span className="text-ok">activa en 127.0.0.1:{bridge.port}</span> : <span className="text-red-400">{bridge.error}</span>}
            {' · '}Plugin: {bridge.plugin_installed ? <span className="text-ok">instalado</span> : <span className="text-amber-300">no instalado</span>}
          </p>
          {confirmPlugin
            ? <div className="mt-3 rounded-xl bg-panel/60 p-3 text-sm text-zinc-300">Se escribirá <span className="font-mono text-xs text-accent">{bridge.plugin_path}</span>{bridge.plugin_installed && ' (reemplaza la versión anterior del plugin de ROBLOX VAULT)'}. Después reinicia Roblox Studio.
                <div className="mt-2 flex gap-2"><button className="btn-primary" onClick={async () => { try { await api.installPlugin(); push('ok', 'Plugin instalado. Reinicia Roblox Studio.'); setConfirmPlugin(false); setBridge(await api.bridgeInfo()) } catch (e) { toastError(e) } }}>Confirmar</button>
                <button className="btn-ghost" onClick={() => setConfirmPlugin(false)}>Cancelar</button></div></div>
            : <button className="btn-primary mt-3" disabled={!bridge.plugin_path} onClick={() => setConfirmPlugin(true)}><Blocks size={14} />{bridge.plugin_installed ? 'Actualizar plugin' : 'Instalar plugin en Studio'}</button>}
          <p className="mt-2 text-xs text-zinc-500">En Studio aparece la pestaña ROBLOX VAULT. La primera vez Studio te pedirá permiso para conectarse a 127.0.0.1: acéptalo.</p>
        </>}
      </div>
      <div className="card mb-4 p-5">
        <h3 className="flex items-center gap-2 font-medium text-white"><PlugZap size={16} className="text-accent" />Conectar Claude Code (MCP)</h3>
        {!mcp ? <div className="skeleton mt-3 h-10" /> : !mcp.exists
          ? <p className="mt-2 text-sm text-amber-300">El servidor MCP no está junto a la app (modo desarrollo). Se instala con la versión descargada de GitHub.</p>
          : <>
            <p className="mt-1 text-sm text-zinc-400">Pega esto una vez en una terminal (PowerShell). Claude Code podrá buscar, recuperar y registrar recursos en esta bóveda.</p>
            <div className="mt-3 flex items-start gap-2"><pre className="flex-1 overflow-x-auto rounded-xl bg-bg/80 p-3 font-mono text-xs text-accent">{mcp.claude_code}</pre>
              <button className="btn-ghost" onClick={() => navigator.clipboard.writeText(mcp.claude_code).then(() => push('ok', 'Comando copiado'))}><Copy size={14} /></button></div>
            <details className="mt-2 text-xs text-zinc-500"><summary className="cursor-pointer">Claude Desktop (claude_desktop_config.json)</summary>
              <pre className="mt-2 overflow-x-auto rounded-xl bg-bg/80 p-3 font-mono">{JSON.stringify(mcp.claude_desktop, null, 2)}</pre></details>
          </>}
      </div>
      <div className="card mb-4 flex items-center justify-between p-5">
        <div><h3 className="font-medium text-white">Registro automático de recursos probados</h3>
          <p className="mt-1 text-sm text-zinc-400">Si está activo, lo que Claude registre con pruebas superadas entra directo como PARTIALLY VERIFIED. Lo no probado siempre pide tu aprobación; lo que falla se guarda como FAILED.</p></div>
        <button onClick={toggleAuto} className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${auto ? 'bg-violet' : 'bg-panel'}`}><span className={`absolute top-0.5 h-5 w-5 rounded-full bg-white transition-all ${auto ? 'left-[22px]' : 'left-0.5'}`} /></button>
      </div>
      <div className="grid grid-cols-2 gap-4">
        <div className="card p-5">
          <h3 className="font-medium text-white">Integridad de archivos</h3>
          <p className="mt-1 text-sm text-zinc-400">Recalcula el SHA-256 de cada original. Comprueba que el archivo no cambió, no que funcione.</p>
          <button className="btn-primary mt-4" disabled={busy} onClick={verify}><ShieldCheck size={16} />{busy ? 'Verificando…' : 'Verificar'}</button>
          {rep && <p className="mt-3 text-sm"><span className="text-ok">{rep.ok}/{rep.checked} correctos</span>
            {rep.missing.length > 0 && <span className="ml-3 text-amber-300">{rep.missing.length} faltan</span>}
            {rep.corrupted.length > 0 && <span className="ml-3 text-red-400">{rep.corrupted.length} corruptos</span>}</p>}
        </div>
        <div className="card p-5">
          <h3 className="font-medium text-white">Copia de seguridad</h3>
          <p className="mt-1 text-sm text-zinc-400">Copia consistente de la base de datos. Los archivos originales son inmutables y no se duplican.</p>
          <button className="btn-ghost mt-4" onClick={backup}><DatabaseBackup size={16} />Crear copia</button>
        </div>
      </div>
    </div>
  )
}
