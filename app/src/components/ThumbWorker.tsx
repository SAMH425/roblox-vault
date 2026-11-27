import { useEffect, useState } from 'react'
import * as THREE from 'three'
import { api, isTauri, type Asset } from '../lib/api'
import { buildRoblox, loadGeneric } from '../lib/models'

/** Genera en segundo plano las miniaturas de los modelos que aún no la tienen (uno a la vez, sin bloquear la UI). */
export function useThumbWorker(onDone: () => void) {
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null)

  useEffect(() => {
    if (!isTauri()) return
    let stop = false
    const renderer = new THREE.WebGLRenderer({ antialias: true, preserveDrawingBuffer: true, alpha: false })
    renderer.setSize(256, 256, false)
    renderer.setPixelRatio(1)
    const scene = new THREE.Scene()
    scene.background = new THREE.Color('#191922')
    scene.add(new THREE.AmbientLight(0xffffff, 0.7))
    const sun = new THREE.DirectionalLight(0xffffff, 1.6); sun.position.set(3, 5, 4); scene.add(sun)
    const fill = new THREE.DirectionalLight(0xc5b5ff, 0.5); fill.position.set(-4, 2, -3); scene.add(fill)
    const camera = new THREE.PerspectiveCamera(40, 1, 0.01, 100000)

    async function shot(a: Asset): Promise<Uint8Array> {
      const m = ['rbxm', 'rbxmx'].includes(a.format) ? buildRoblox(await api.scene(a.id)) : await loadGeneric(await api.blob(a.id), a.format)
      const obj = m.object
      obj.updateMatrixWorld(true)
      scene.add(obj)
      const box = new THREE.Box3().setFromObject(obj)
      const size = box.getSize(new THREE.Vector3()), center = box.getCenter(new THREE.Vector3())
      const radius = Math.max(size.length() / 2, 0.01)
      const dist = radius / Math.sin(THREE.MathUtils.degToRad(camera.fov / 2)) * 1.05
      camera.position.copy(center).add(new THREE.Vector3(1, 0.75, 1.2).normalize().multiplyScalar(dist))
      camera.near = dist / 100; camera.far = dist * 10; camera.updateProjectionMatrix()
      camera.lookAt(center)
      renderer.render(scene, camera)
      scene.remove(obj)
      obj.traverse((c) => {
        const mesh = c as THREE.Mesh
        if (mesh.isMesh) { mesh.geometry?.dispose(); for (const mt of Array.isArray(mesh.material) ? mesh.material : [mesh.material]) mt?.dispose() }
      })
      const blob = await new Promise<Blob>((r) => renderer.domElement.toBlob((b) => r(b!), 'image/png'))
      return new Uint8Array(await blob.arrayBuffer())
    }

    ;(async () => {
      await new Promise((r) => setTimeout(r, 2500)) // deja arrancar la app primero
      const queue = await api.missingThumbs().catch(() => [] as Asset[])
      if (!queue.length || stop) return
      setProgress({ done: 0, total: queue.length })
      for (let i = 0; i < queue.length && !stop; i++) {
        const a = queue[i]
        try { await api.saveVersionThumb(a.id, a.version, await shot(a)) }
        catch { await api.updateAsset(a.id, { metadata: { ...a.metadata, thumb_failed: true } }).catch(() => {}) } // no reintentar en bucle
        setProgress({ done: i + 1, total: queue.length })
        if ((i + 1) % 6 === 0) onDone()
        await new Promise((r) => setTimeout(r, 30))
      }
      onDone()
      setTimeout(() => setProgress(null), 3000)
    })()
    return () => { stop = true; renderer.dispose() }
  }, [])
  return progress
}
