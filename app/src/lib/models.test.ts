import { describe, expect, it } from 'vitest'
import { buildRoblox, loadGeneric, stats, type RbxModel } from './models'

const I = [1, 0, 0, 0, 1, 0, 0, 0, 1]
describe('modelos', () => {
  it('convierte una escena de Roblox en mallas con tamaño real', () => {
    const m: RbxModel = { tree: [{ name: 'Lamp', class: 'Model', children: [] }], scripts: [], instance_count: 3, class_counts: {}, materials: {}, bounds_min: [], bounds_max: [],
      parts: [
        { path: 'Lamp/Base', class: 'Part', shape: 'block', size: [4, 1, 4], position: [0, 0.5, 0], rotation: I, color: [255, 0, 0], material: 'Metal', transparency: 0, mesh_id: null },
        { path: 'Lamp/Bulb', class: 'Part', shape: 'ball', size: [2, 2, 2], position: [0, 3, 0], rotation: I, color: [255, 255, 255], material: 'Neon', transparency: 0, mesh_id: null },
        { path: 'Lamp/Ghost', class: 'Part', shape: 'block', size: [1, 1, 1], position: [0, 0, 0], rotation: I, color: [0, 0, 0], material: 'Plastic', transparency: 1, mesh_id: null },
        { path: 'Lamp/Mesh', class: 'MeshPart', shape: 'mesh', size: [1, 1, 1], position: [0, 0, 0], rotation: I, color: [0, 0, 0], material: 'Plastic', transparency: 0, mesh_id: 'rbxassetid://1' },
      ] }
    const r = buildRoblox(m)
    r.object.updateMatrixWorld(true)
    const s = stats(r.object)
    expect(s.meshes).toBe(3) // la invisible no se dibuja
    expect(s.size).toEqual([4, 4.5, 4]) // la MeshPart en el origen baja hasta -0.5
    expect(s.materials).toContain('Metal')
    expect(r.note).toMatch(/MeshPart/)
  })
  it('lee OBJ', async () => {
    const obj = 'v 0 0 0\nv 2 0 0\nv 0 3 0\nf 1 2 3\n'
    const r = await loadGeneric(new TextEncoder().encode(obj).buffer as ArrayBuffer, 'obj')
    const s = stats(r.object)
    expect(s.meshes).toBe(1); expect(s.triangles).toBe(1); expect(s.size).toEqual([2, 3, 0])
  })
})

describe('cuña', () => {
  it('lado alto atrás (+Z)', () => {
    const I = [1, 0, 0, 0, 1, 0, 0, 0, 1]
    const r = buildRoblox({ tree: [], scripts: [], instance_count: 1, class_counts: {}, materials: {}, bounds_min: [], bounds_max: [],
      parts: [{ path: 'W', class: 'WedgePart', shape: 'wedge', size: [1, 1, 1], position: [0, 0, 0], rotation: I, color: [0, 0, 0], material: 'Plastic', transparency: 0, mesh_id: null }] })
    const pos = (r.object.children[0] as any).geometry.attributes.position
    let top = -1
    for (let i = 0; i < pos.count; i++) if (pos.getY(i) > 0.49) top = Math.max(top, pos.getZ(i))
    expect(top).toBeCloseTo(0.5)
    expect(stats(r.object).size).toEqual([1, 1, 1])
  })
})
