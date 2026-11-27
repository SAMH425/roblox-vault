import * as THREE from 'three'
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js'
import { OBJLoader } from 'three/examples/jsm/loaders/OBJLoader.js'
import { FBXLoader } from 'three/examples/jsm/loaders/FBXLoader.js'

export interface RbxPart { path: string; class: string; shape: string; size: [number, number, number]; position: [number, number, number]; rotation: number[]; color: [number, number, number]; material: string; transparency: number; mesh_id: string | null }
export interface RbxNode { name: string; class: string; children: RbxNode[] }
export interface RbxModel { tree: RbxNode[]; parts: RbxPart[]; scripts: { path: string; class: string; source: string }[]; instance_count: number; class_counts: Record<string, number>; materials: Record<string, number>; bounds_min: number[]; bounds_max: number[] }

export interface LoadedModel { object: THREE.Object3D; tree: RbxNode[]; note?: string }

/** Cuña de Roblox: rampa que sube hacia -Z (frente) en el eje Y. */
function wedgeGeometry() {
  const s = new THREE.Shape()
  s.moveTo(-0.5, -0.5); s.lineTo(0.5, -0.5); s.lineTo(-0.5, 0.5); s.closePath()
  const g = new THREE.ExtrudeGeometry(s, { depth: 1, bevelEnabled: false })
  g.translate(0, 0, -0.5)
  g.rotateY(Math.PI / 2) // lado alto atrás (+Z), rampa hacia el frente (-Z) como en Roblox
  return g
}
function cornerWedgeGeometry() {
  const v = new Float32Array([
    -0.5, -0.5, -0.5, 0.5, -0.5, -0.5, 0.5, -0.5, 0.5, -0.5, -0.5, 0.5, 0.5, 0.5, -0.5,
  ])
  const idx = [0, 2, 1, 0, 3, 2, 0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4]
  const g = new THREE.BufferGeometry()
  g.setAttribute('position', new THREE.BufferAttribute(v, 3)); g.setIndex(idx); g.computeVertexNormals()
  return g
}

const MAT_STYLE: Record<string, { roughness: number; metalness: number; emissive?: boolean }> = {
  Metal: { roughness: 0.35, metalness: 0.9 }, DiamondPlate: { roughness: 0.3, metalness: 0.9 }, CorrodedMetal: { roughness: 0.8, metalness: 0.6 },
  Foil: { roughness: 0.2, metalness: 1 }, Glass: { roughness: 0.05, metalness: 0 }, Ice: { roughness: 0.1, metalness: 0 },
  SmoothPlastic: { roughness: 0.35, metalness: 0 }, Neon: { roughness: 1, metalness: 0, emissive: true },
}

/** Convierte la escena extraída de RBXM/RBXMX en objetos Three.js (1 stud = 1 unidad). */
export function buildRoblox(m: RbxModel): LoadedModel {
  const group = new THREE.Group(); group.name = m.tree[0]?.name ?? 'Model'
  const geos: Record<string, THREE.BufferGeometry> = {
    block: new THREE.BoxGeometry(1, 1, 1), ball: new THREE.SphereGeometry(0.5, 32, 16),
    cylinder: new THREE.CylinderGeometry(0.5, 0.5, 1, 32).rotateZ(Math.PI / 2), // en Roblox el cilindro va en X
    wedge: wedgeGeometry(), cornerwedge: cornerWedgeGeometry(), mesh: new THREE.BoxGeometry(1, 1, 1),
  }
  for (const p of m.parts) {
    if (p.transparency >= 1) continue
    const style = MAT_STYLE[p.material] ?? { roughness: 0.6, metalness: 0 }
    const color = new THREE.Color(p.color[0] / 255, p.color[1] / 255, p.color[2] / 255)
    const mat = new THREE.MeshStandardMaterial({
      color, roughness: style.roughness, metalness: style.metalness, transparent: p.transparency > 0, opacity: 1 - p.transparency,
      emissive: style.emissive ? color : undefined, emissiveIntensity: style.emissive ? 1.2 : 0, wireframe: p.shape === 'mesh',
    })
    mat.name = p.material
    const mesh = new THREE.Mesh(geos[p.shape] ?? geos.block, mat)
    mesh.name = p.path
    mesh.userData = { class: p.class, material: p.material, meshId: p.mesh_id }
    const r = p.rotation
    const m4 = new THREE.Matrix4().set(r[0], r[1], r[2], p.position[0], r[3], r[4], r[5], p.position[1], r[6], r[7], r[8], p.position[2], 0, 0, 0, 1)
    m4.multiply(new THREE.Matrix4().makeScale(p.size[0], p.size[1], p.size[2]))
    mesh.matrixAutoUpdate = false; mesh.matrix.copy(m4)
    mesh.castShadow = mesh.receiveShadow = true
    group.add(mesh)
  }
  group.updateMatrixWorld(true)
  const meshParts = m.parts.filter((p) => p.shape === 'mesh').length
  return { object: group, tree: m.tree, note: meshParts ? `${meshParts} MeshPart/Union se muestran como caja en alambre: su malla está en la nube de Roblox, no dentro del archivo.` : undefined }
}

function treeOf(o: THREE.Object3D, depth = 0): RbxNode {
  return { name: o.name || o.type, class: o.type, children: depth > 12 ? [] : o.children.map((c) => treeOf(c, depth + 1)) }
}

/** Carga GLB/GLTF/OBJ/FBX desde bytes. */
export async function loadGeneric(buf: ArrayBuffer, format: string): Promise<LoadedModel> {
  let object: THREE.Object3D
  let note: string | undefined
  if (format === 'glb' || format === 'gltf') {
    const gltf = await new GLTFLoader().parseAsync(buf, '')
    object = gltf.scene
    if (format === 'gltf' && !new TextDecoder().decode(buf).includes('data:')) note = 'Si el .gltf usa archivos .bin o texturas externas, impórtalo como .glb para verlo completo.'
  } else if (format === 'obj') {
    object = new OBJLoader().parse(new TextDecoder().decode(buf))
  } else if (format === 'fbx') {
    object = new FBXLoader().parse(buf, '')
  } else throw new Error(`Formato no soportado por el visor: ${format}`)
  object.traverse((c) => { if ((c as THREE.Mesh).isMesh) { c.castShadow = c.receiveShadow = true } })
  return { object, tree: [treeOf(object)], note }
}

export function stats(o: THREE.Object3D) {
  const box = new THREE.Box3().setFromObject(o)
  const size = box.getSize(new THREE.Vector3())
  let meshes = 0, triangles = 0
  const materials = new Set<string>()
  o.traverse((c) => {
    const m = c as THREE.Mesh
    if (!m.isMesh) return
    meshes++
    const g = m.geometry
    triangles += g.index ? g.index.count / 3 : (g.attributes.position?.count ?? 0) / 3
    for (const mat of Array.isArray(m.material) ? m.material : [m.material]) materials.add(mat.name || mat.type)
  })
  return { size: [size.x, size.y, size.z].map((n) => Math.round(n * 100) / 100), meshes, triangles: Math.round(triangles), materials: [...materials] }
}
