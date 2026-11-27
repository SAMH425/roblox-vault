/** Resaltador Luau mínimo y sin dependencias. Devuelve tokens; nunca HTML sin escapar. */
export type Tok = { t: 'kw' | 'str' | 'com' | 'num' | 'fn' | 'type' | 'plain'; v: string }

const KW = new Set('and break continue do else elseif end export false for function if in local nil not or repeat return then true type typeof until while'.split(' '))
const GLOBALS = new Set('game workspace script Instance Vector3 Vector2 CFrame Color3 UDim2 UDim Enum task math string table os pcall error print warn require tostring tonumber pairs ipairs setmetatable getmetatable typeof'.split(' '))

export function tokenize(src: string): Tok[] {
  const out: Tok[] = []
  let i = 0
  const push = (t: Tok['t'], v: string) => out.push({ t, v })
  while (i < src.length) {
    const c = src[i], rest = src.slice(i)
    let m: RegExpMatchArray | null
    if ((m = rest.match(/^--\[(=*)\[[\s\S]*?\]\1\]/)) || (m = rest.match(/^--[^\n]*/))) { push('com', m[0]) }
    else if ((m = rest.match(/^\[(=*)\[[\s\S]*?\]\1\]/))) { push('str', m[0]) }
    else if (c === '"' || c === "'" || c === '`') {
      let j = i + 1
      while (j < src.length && src[j] !== c && src[j] !== '\n') j += src[j] === '\\' ? 2 : 1
      m = [src.slice(i, j + 1)] as RegExpMatchArray; push('str', m[0])
    }
    else if ((m = rest.match(/^0x[\da-fA-F_]+|^\d[\d_]*(\.\d+)?([eE][+-]?\d+)?/))) { push('num', m[0]) }
    else if ((m = rest.match(/^[A-Za-z_]\w*/))) {
      const w = m[0]
      push(KW.has(w) ? 'kw' : GLOBALS.has(w) ? 'type' : /^\s*[(:"{]/.test(src.slice(i + w.length)) ? 'fn' : 'plain', w)
    }
    else { m = [c] as RegExpMatchArray; push('plain', c) }
    i += m[0].length
  }
  return out
}

export const TOKEN_CLASS: Record<Tok['t'], string> = {
  kw: 'text-[#C5B5FF]', str: 'text-[#36C991]', com: 'text-zinc-500 italic', num: 'text-amber-300', fn: 'text-sky-300', type: 'text-[#9065FF]', plain: 'text-zinc-200',
}

/** Diff por líneas (LCS). Suficiente para scripts; limita tamaño para no bloquear la UI. */
export type DiffLine = { kind: 'same' | 'add' | 'del'; text: string }
export function diffLines(a: string, b: string): DiffLine[] {
  const A = a.split('\n'), B = b.split('\n')
  if (A.length * B.length > 4_000_000) return [{ kind: 'del', text: '(archivos demasiado grandes para comparar)' }]
  const dp = Array.from({ length: A.length + 1 }, () => new Uint32Array(B.length + 1))
  for (let i = A.length - 1; i >= 0; i--) for (let j = B.length - 1; j >= 0; j--)
    dp[i][j] = A[i] === B[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1])
  const out: DiffLine[] = []
  let i = 0, j = 0
  while (i < A.length && j < B.length) {
    if (A[i] === B[j]) { out.push({ kind: 'same', text: A[i] }); i++; j++ }
    else if (dp[i + 1][j] >= dp[i][j + 1]) out.push({ kind: 'del', text: A[i++] })
    else out.push({ kind: 'add', text: B[j++] })
  }
  while (i < A.length) out.push({ kind: 'del', text: A[i++] })
  while (j < B.length) out.push({ kind: 'add', text: B[j++] })
  return out
}
