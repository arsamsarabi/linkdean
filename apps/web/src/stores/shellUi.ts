import { create } from 'zustand'

/** Ephemeral AppShell chrome — drawers / palette. Not server state. */
type ShellUiState = {
  paletteOpen: boolean
  notifOpen: boolean
  historyOpen: boolean
  brokenOpen: boolean
  navOpen: boolean
  openPalette: () => void
  closePalette: () => void
  togglePalette: () => void
  openNotif: () => void
  closeNotif: () => void
  openHistory: () => void
  closeHistory: () => void
  openBroken: () => void
  closeBroken: () => void
  toggleNav: () => void
  closeNav: () => void
}

export const useShellUi = create<ShellUiState>((set) => ({
  paletteOpen: false,
  notifOpen: false,
  historyOpen: false,
  brokenOpen: false,
  navOpen: false,
  openPalette: () => set({ paletteOpen: true }),
  closePalette: () => set({ paletteOpen: false }),
  togglePalette: () => set((s) => ({ paletteOpen: !s.paletteOpen })),
  openNotif: () => set({ notifOpen: true }),
  closeNotif: () => set({ notifOpen: false }),
  openHistory: () => set({ historyOpen: true }),
  closeHistory: () => set({ historyOpen: false }),
  openBroken: () => set({ brokenOpen: true }),
  closeBroken: () => set({ brokenOpen: false }),
  toggleNav: () => set((s) => ({ navOpen: !s.navOpen })),
  closeNav: () => set({ navOpen: false }),
}))
