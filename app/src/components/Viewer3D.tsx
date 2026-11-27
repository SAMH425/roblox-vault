import { Suspense, useEffect, useMemo, useRef, useState } from 'react'
import { Canvas, useThree } from '@react-three/fiber'
import { Bounds, Environment, Grid, Lightformer, OrbitControls, useBounds } from '@react-three/drei'
import * as THREE from 'three'
import { Box, Camera, ChevronRight, Expand, Image as ImageIcon, Maximize2, Minimize2, Ruler, Sun } from 'lucide-react'
import { api, type Asset } from '../lib/api'
import { buildRoblox, loadGeneric, stats, type LoadedModel, type RbxNode } from '../lib/models'
import { toastError, useToast } from '../state/toast'

const ROBLOX = ['rbxm', 'rbxmx']
export const VIEWABLE = ['glb', 'gltf', 'obj', 'fbx', ...ROBLOX]

async function load(asset: Asset, version?: number): Promise<LoadedModel> {
  if (ROBLOX.includes(asset.format)) return buildRoblox(await api.scene(asset.id, version))
  return loadGeneric(await api.blob(asset.id, version), asset.format)
}

function FitControls({ fitSignal }: { fitSignal: number }) {
  const bounds = useBounds()
  useEffect(() => { bounds.refresh().clip().fit() }, [fitSignal, bounds])
  return null
}

/** Captura una miniatura PNG de 256 px del canvas actual. */
function Snapshot({ onReady }: { onReady: (fn: () => Promise<Uint8Array>) => void }) {
  const { gl, scene, camera } = useThree()
  useEffect(() => {
    onReady(async () => {
      gl.render(scene, camera)
      const src = gl.domElement
      const c = document.createElement('canvas'); c.width = c.height = 256
      const ctx = c.getContext('2d')!
      ctx.fillStyle = '#191922'; ctx.fillRect(0, 0, 256, 256)
      const s = Math.min(src.width, src.height)
      ctx.drawImage(src, (src.width - s) / 2, (src.height - s) / 2, s, s, 0, 0, 256, 256)
      const blob = await new Promise<Blob>((r) => c.toBlob((b) => r(b!), 'image/png'))
      return new Uint8Array(await blob.arrayBuffer())
    })
  }, [gl, scene, camera, onReady])
  return null
}

export function Viewer3D({ asset, version, onMeta, onThumb }: { asset: Asset; version?: number; onMeta?: (m: Record<string, unknown>) => void; onThumb?: () => void }) {
  const [model, setModel] = useState<LoadedModel | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [light, setLight] = useState(1.2)
  const [env, setEnv] = useState(true)
  const [wire, setWire] = useState(false)
  const [fit, setFit] = useState(0)
  const [full, setFull] = useState(false)
  const [showTree, setShowTree] = useState(false)
  const controls = useRef<any>(null)
  const snap = useRef<(() => Promise<Uint8Array>) | null>(null)
  const push = useToast((s) => s.push)

  useEffect(() => {
    let alive = true
    setModel(null); setErr(null)
    load(asset, version).then((m) => { if (alive) setModel(m) }).catch((e) => { if (alive) setErr(String(e?.message ?? e)) })
    return () => { alive = false }
  }, [asset.id, asset.format, version])

  const info = useMemo(() => (model ? stats(model.object) : null), [model])

  useEffect(() => {
    if (!model || !info) return
    model.object.traverse((c) => {
      const m = (c as THREE.Mesh).material
      for (const mat of m ? (Array.isArray(m) ? m : [m]) : []) (mat as THREE.MeshStandardMaterial).wireframe = wire || (c.userData.class === 'MeshPart')
    })
  }, [wire, model, info])

  // primera carga: completar metadatos (solo formatos no Roblox; esos ya se extraen al importar) y crear miniatura si falta
  useEffect(() => {
    if (!model || !info) return
    if (onMeta && !ROBLOX.includes(asset.format) && asset.metadata.meshes === undefined)
      onMeta({ ...asset.metadata, meshes: info.meshes, triangles: info.triangles, dimensions: info.size, materials: info.materials })
    const t = setTimeout(async () => {
      const existing = await api.thumb(asset.id).catch(() => null)
      if (!existing && snap.current) { await api.saveThumb(asset.id, await snap.current()).catch(() => {}); onThumb?.() }
    }, 900)
    return () => clearTimeout(t)
  }, [model, info])

  async function updateThumb() {
    try { if (snap.current) { await api.saveThumb(asset.id, await snap.current()); push('ok', 'Miniatura actualizada'); onThumb?.() } } catch (e) { toastError(e) }
  }
  function resetCamera() { controls.current?.reset(); setFit((f) => f + 1) }

  const shell = full ? 'fixed inset-4 z-50 shadow-2xl' : 'relative h-[380px]'
  return (
    <div className={`${shell} overflow-hidden rounded-2xl border border-white/[0.06] bg-[#0c0c10]`}>
      {err ? <div className="grid h-full place-items-center p-6 text-center text-sm text-red-300">{err}</div>
        : !model ? <div className="grid h-full place-items-center"><div className="h-8 w-8 animate-spin rounded-full border-2 border-violet border-t-transparent" /></div>
        : (
          <Canvas shadows gl={{ preserveDrawingBuffer: true, antialias: true }} camera={{ position: [8, 6, 10], fov: 45, near: 0.05, far: 50000 }} dpr={[1, 2]}>
            <color attach="background" args={['#0c0c10']} />
            <ambientLight intensity={0.35 * light} />
            <directionalLight position={[30, 50, 20]} intensity={light} castShadow shadow-mapSize={[2048, 2048]} />
            <Suspense fallback={null}>{env && (
              // entorno generado localmente (sin descargar HDRIs de internet)
              <Environment resolution={128} environmentIntensity={0.6 * light}>
                <Lightformer intensity={2} position={[0, 5, -9]} scale={[10, 10, 1]} />
                <Lightformer intensity={1} color="#C5B5FF" position={[-5, 1, -1]} rotation-y={Math.PI / 2} scale={[20, 2, 1]} />
                <Lightformer intensity={1} position={[10, 1, 0]} rotation-y={-Math.PI / 2} scale={[20, 2, 1]} />
              </Environment>)}</Suspense>
            <Bounds fit clip observe margin={1.25}>
              <primitive object={model.object} />
              <FitControls fitSignal={fit} />
            </Bounds>
            <Grid infiniteGrid fadeDistance={200} sectionColor="#282535" cellColor="#1d1b27" position={[0, (info ? new THREE.Box3().setFromObject(model.object).min.y : 0) - 0.01, 0]} />
            <OrbitControls ref={controls} makeDefault enableDamping />
            <Snapshot onReady={(fn) => { snap.current = fn }} />
          </Canvas>
        )}

      {model && info && <>
        <div className="absolute top-3 left-3 flex gap-1.5">
          <Btn title="Ajustar a pantalla" onClick={() => setFit((f) => f + 1)}><Maximize2 size={14} /></Btn>
          <Btn title="Restablecer cámara" onClick={resetCamera}><Camera size={14} /></Btn>
          <Btn title="Alambre / materiales" active={wire} onClick={() => setWire(!wire)}><Box size={14} /></Btn>
          <Btn title="Entorno de iluminación" active={env} onClick={() => setEnv(!env)}><Sun size={14} /></Btn>
          <Btn title="Jerarquía" active={showTree} onClick={() => setShowTree(!showTree)}><ChevronRight size={14} /></Btn>
          <Btn title="Actualizar miniatura" onClick={updateThumb}><ImageIcon size={14} /></Btn>
          <Btn title={full ? 'Salir' : 'Pantalla completa'} onClick={() => setFull(!full)}>{full ? <Minimize2 size={14} /> : <Expand size={14} />}</Btn>
        </div>
        <div className="absolute top-3 right-3 flex items-center gap-2 rounded-lg bg-black/50 px-2 py-1 text-[11px] text-zinc-300 backdrop-blur">
          <Sun size={12} /><input type="range" min={0.2} max={3} step={0.1} value={light} onChange={(e) => setLight(+e.target.value)} className="w-20 accent-[#9065FF]" />
        </div>
        <div className="absolute bottom-3 left-3 flex gap-3 rounded-lg bg-black/50 px-3 py-1.5 text-[11px] text-zinc-300 backdrop-blur">
          <span className="flex items-center gap-1"><Ruler size={12} />{info.size.join(' × ')}{ROBLOX.includes(asset.format) ? ' studs' : ''}</span>
          <span>{info.meshes} piezas</span><span>{info.triangles.toLocaleString()} triángulos</span>
        </div>
        {model.note && <div className="absolute right-3 bottom-3 max-w-[50%] rounded-lg bg-amber-400/10 px-2 py-1 text-[11px] text-amber-200">{model.note}</div>}
        {showTree && <div className="absolute top-12 left-3 max-h-[70%] w-60 overflow-auto rounded-xl bg-black/70 p-2 text-[11px] backdrop-blur">{model.tree.map((n, i) => <Tree key={i} n={n} />)}</div>}
      </>}
    </div>
  )
}

function Tree({ n, d = 0 }: { n: RbxNode; d?: number }) {
  const [open, setOpen] = useState(d < 2)
  return (
    <div style={{ paddingLeft: d ? 10 : 0 }}>
      <button className="flex w-full items-center gap-1 truncate py-0.5 text-left hover:text-white" onClick={() => setOpen(!open)}>
        {n.children.length > 0 ? <ChevronRight size={10} className={open ? 'rotate-90' : ''} /> : <span className="w-[10px]" />}
        <span className="text-zinc-200">{n.name}</span><span className="text-zinc-500">{n.class}</span>
      </button>
      {open && n.children.map((c, i) => <Tree key={i} n={c} d={d + 1} />)}
    </div>
  )
}

function Btn({ children, onClick, title, active }: { children: React.ReactNode; onClick: () => void; title: string; active?: boolean }) {
  return <button title={title} onClick={onClick} className={`rounded-lg p-1.5 backdrop-blur transition-colors ${active ? 'bg-violet/40 text-white' : 'bg-black/50 text-zinc-300 hover:bg-black/70 hover:text-white'}`}>{children}</button>
}

