import { create } from 'zustand'

const NAV_COLLAPSED_KEY = 'linkdean-sidebar-collapsed'
const WORKSPACE_KEY = 'linkdean-workspace'

export type WorkspaceSelection =
  { kind: 'personal' } | { kind: 'organization'; organizationId: string; teamId: string | null }

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

function readWorkspace(): WorkspaceSelection {
  try {
    const raw = localStorage.getItem(WORKSPACE_KEY)
    if (!raw) return { kind: 'personal' }
    const parsed = JSON.parse(raw) as WorkspaceSelection
    if (parsed?.kind === 'organization' && typeof parsed.organizationId === 'string') {
      return {
        kind: 'organization',
        organizationId: parsed.organizationId,
        teamId: typeof parsed.teamId === 'string' ? parsed.teamId : null,
      }
    }
  } catch {
    // ignore
  }
  return { kind: 'personal' }
}

function writeWorkspace(value: WorkspaceSelection): void {
  try {
    localStorage.setItem(WORKSPACE_KEY, JSON.stringify(value))
  } catch {
    // Storage blocked: the choice lasts until reload.
  }
}

/** Ephemeral AppShell chrome — drawers / palette. Not server state. */
type ShellUiState = {
  paletteOpen: boolean
  notifOpen: boolean
  navOpen: boolean
  /** Wide screens only: the sidebar is an icon rail. Remembered across reloads. */
  navCollapsed: boolean
  /** Personal library vs an organization workspace (+ optional team). */
  workspace: WorkspaceSelection
  openPalette: () => void
  closePalette: () => void
  togglePalette: () => void
  openNotif: () => void
  closeNotif: () => void
  toggleNav: () => void
  closeNav: () => void
  toggleNavCollapsed: () => void
  setPersonalWorkspace: () => void
  setOrganizationWorkspace: (organizationId: string, teamId?: string | null) => void
  setWorkspaceTeam: (teamId: string | null) => void
}

export const useShellUi = create<ShellUiState>((set) => ({
  paletteOpen: false,
  notifOpen: false,
  navOpen: false,
  navCollapsed: readNavCollapsed(),
  workspace: readWorkspace(),
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
  setPersonalWorkspace: () => {
    const workspace: WorkspaceSelection = { kind: 'personal' }
    writeWorkspace(workspace)
    set({ workspace })
  },
  setOrganizationWorkspace: (organizationId, teamId = null) => {
    const workspace: WorkspaceSelection = { kind: 'organization', organizationId, teamId }
    writeWorkspace(workspace)
    set({ workspace })
  },
  setWorkspaceTeam: (teamId) =>
    set((s) => {
      if (s.workspace.kind !== 'organization') return s
      const workspace: WorkspaceSelection = { ...s.workspace, teamId }
      writeWorkspace(workspace)
      return { workspace }
    }),
}))
