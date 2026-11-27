import { useEffect, useState } from 'react'
import { api, type Asset } from '../lib/api'
import { useThumbBus } from '../state/thumbs'

/** Miniatura real guardada en la bóveda (o el formato si aún no existe). */
export function Thumb({ asset, bump = 0 }: { asset: Asset; bump?: number }) {
  const [url, setUrl] = useState<string | null>(null)
  const n = useThumbBus((s) => s.n)
  useEffect(() => {
    let u: string | null = null
    const src = asset.kind === 'texture' ? api.blob(asset.id).then((b) => (b.byteLength ? b : null)) : api.thumb(asset.id)
    src.then((b) => { if (b) { u = URL.createObjectURL(new Blob([b], { type: asset.format === 'jpg' || asset.format === 'jpeg' ? 'image/jpeg' : 'image/png' })); setUrl(u) } }).catch(() => {})
    return () => { if (u) URL.revokeObjectURL(u) }
  }, [asset.id, asset.version, bump, url ? 0 : n])
  return url ? <img src={url} className={`h-full w-full ${asset.kind === 'texture' ? 'object-contain p-2' : 'object-cover'}`} /> : <span className="text-2xl font-semibold text-accent/70 uppercase">{asset.format}</span>
}
