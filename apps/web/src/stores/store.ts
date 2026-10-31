import { create } from 'zustand'

/**
 * UI mirror of server prefs only — not source of truth.
 * Synced from usePrefsHydration (/auth/me) and settings/vault mutations that invalidate ['me'].
 * Do not write showHidden/vaultUnlocked from ad-hoc UI without also updating server state.
 */
type UiState = {
  showHidden: boolean
  vaultUnlocked: boolean
  setShowHidden: (v: boolean) => void
  setVaultUnlocked: (v: boolean) => void
}

export const useUi = create<UiState>((set) => ({
  showHidden: false,
  vaultUnlocked: false,
  setShowHidden: (showHidden) => set({ showHidden }),
  setVaultUnlocked: (vaultUnlocked) => set({ vaultUnlocked }),
}))
