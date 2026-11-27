import { describe, expect, it } from 'vitest'
import { diffLines, tokenize } from './luau'

describe('luau', () => {
  it('tokeniza sin perder texto', () => {
    const src = 'local x = "hi" -- c\nfunction f() return 0x1F end\n--[[ b ]] game:GetService("X")'
    expect(tokenize(src).map((t) => t.v).join('')).toBe(src)
    expect(tokenize('local').at(0)?.t).toBe('kw')
    expect(tokenize('-- hola')[0].t).toBe('com')
  })
  it('diff', () => {
    const d = diffLines('a\nb\nc', 'a\nx\nc')
    expect(d.filter((l) => l.kind === 'del').map((l) => l.text)).toEqual(['b'])
    expect(d.filter((l) => l.kind === 'add').map((l) => l.text)).toEqual(['x'])
  })
})
