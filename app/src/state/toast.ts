import { create } from 'zustand'

export interface Toast { id: number; kind: 'ok' | 'error' | 'info'; text: string }
interface S { toasts: Toast[]; push: (kind: Toast['kind'], text: string) => void; drop: (id: number) => void }

let seq = 0
export const useToast = create<S>((set) => ({
  toasts: [],
  push: (kind, text) => {
    const id = ++seq
    set((s) => ({ toasts: [...s.toasts, { id, kind, text }] }))
    setTimeout(() => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })), 4500)
  },
  drop: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}))

export const toastError = (e: unknown) => useToast.getState().push('error', e instanceof Error ? e.message : String(e))
