import { create } from 'zustand'

/** Aviso global: hay miniaturas nuevas (las tarjetas se recargan). */
export const useThumbBus = create<{ n: number; bump: () => void }>((set) => ({ n: 0, bump: () => set((s) => ({ n: s.n + 1 })) }))
