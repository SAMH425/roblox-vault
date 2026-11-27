import { useMemo } from 'react'
import { diffLines, tokenize, TOKEN_CLASS } from '../lib/luau'

export function CodeView({ code }: { code: string }) {
  const lines = useMemo(() => {
    const rows: { t: keyof typeof TOKEN_CLASS; v: string }[][] = [[]]
    for (const tok of tokenize(code)) {
      const parts = tok.v.split('\n')
      parts.forEach((p, i) => { if (i > 0) rows.push([]); if (p) rows[rows.length - 1].push({ t: tok.t, v: p }) })
    }
    return rows
  }, [code])
  return (
    <pre className="overflow-auto rounded-xl bg-bg/80 p-3 font-mono text-[12.5px] leading-5">
      {lines.map((l, i) => (
        <div key={i} className="flex"><span className="w-10 shrink-0 pr-3 text-right text-zinc-600 select-none">{i + 1}</span>
          <span>{l.map((t, j) => <span key={j} className={TOKEN_CLASS[t.t]}>{t.v}</span>)}</span></div>
      ))}
    </pre>
  )
}

export function DiffView({ a, b }: { a: string; b: string }) {
  const d = useMemo(() => diffLines(a, b), [a, b])
  const style = { same: 'text-zinc-400', add: 'bg-ok/10 text-ok', del: 'bg-red-500/10 text-red-400' }
  const sign = { same: ' ', add: '+', del: '-' }
  return (
    <pre className="overflow-auto rounded-xl bg-bg/80 p-3 font-mono text-[12.5px] leading-5">
      {d.map((l, i) => <div key={i} className={style[l.kind]}>{sign[l.kind]} {l.text}</div>)}
    </pre>
  )
}
