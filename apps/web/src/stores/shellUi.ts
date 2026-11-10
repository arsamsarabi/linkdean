import { create } from 'zustand'

const NAV_COLLAPSED_KEY = 'linkdean-sidebar-collapsed'

function readNavCollapsed(): boolean {
  try {
    return localStorage.getItem(NAV_COLLAPSED_KEY) === 'true'
  } catch {
    return false
  }
}

function writeNavCollapsed(value: boolean): void {
  try {
    localStorage.setItem(NAV_COLLAPSED_KEY, String(value))
  } catch {
    // Storage blocked: the choice lasts until reload.
  }
}

/** Ephemeral AppShell chrome — drawers / palette. Not server state. */
type ShellUiState = {
  paletteOpen: boolean
  notifOpen: boolean
  navOpen: boolean
  /** Wide screens only: the sidebar is hidden. Remembered across reloads. */
  navCollapsed: boolean
  openPalette: () => void
  closePalette: () => void
  togglePalette: () => void
  openNotif: () => void
  closeNotif: () => void
  toggleNav: () => void
  closeNav: () => void
  toggleNavCollapsed: () => void
}

export const useShellUi = create<ShellUiState>((set) => ({
  paletteOpen: false,
  notifOpen: false,
  navOpen: false,
  navCollapsed: readNavCollapsed(),
  openPalette: () => set({ paletteOpen: true }),
  closePalette: () => set({ paletteOpen: false }),
  togglePalette: () => set((s) => ({ paletteOpen: !s.paletteOpen })),
  openNotif: () => set({ notifOpen: true }),
  closeNotif: () => set({ notifOpen: false }),
  toggleNav: () => set((s) => ({ navOpen: !s.navOpen })),
  closeNav: () => set({ navOpen: false }),
  toggleNavCollapsed: () =>
    set((s) => {
      writeNavCollapsed(!s.navCollapsed)
      return { navCollapsed: !s.navCollapsed }
    }),
}))
