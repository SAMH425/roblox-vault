import { useEffect, useState } from 'react'
import { api, type Asset } from '../lib/api'

/** Miniatura real guardada en la bóveda (o el formato si aún no existe). */
export function Thumb({ asset, bump = 0 }: { asset: Asset; bump?: number }) {
  const [url, setUrl] = useState<string | null>(null)
  useEffect(() => {
    let u: string | null = null
    api.thumb(asset.id).then((b) => { if (b) { u = URL.createObjectURL(new Blob([b], { type: 'image/png' })); setUrl(u) } }).catch(() => {})
    return () => { if (u) URL.revokeObjectURL(u) }
  }, [asset.id, asset.version, bump])
  return url ? <img src={url} className="h-full w-full object-cover" /> : <span className="text-2xl font-semibold text-accent/70 uppercase">{asset.format}</span>
}
