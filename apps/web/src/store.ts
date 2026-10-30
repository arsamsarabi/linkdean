import { create } from 'zustand'

type UiState = {
  showHidden: boolean
  vaultUnlocked: boolean
  selectedCategoryId: string | null
  setShowHidden: (v: boolean) => void
  setVaultUnlocked: (v: boolean) => void
  setSelectedCategoryId: (id: string | null) => void
}

export const useUi = create<UiState>((set) => ({
  showHidden: false,
  vaultUnlocked: false,
  selectedCategoryId: null,
  setShowHidden: (showHidden) => set({ showHidden }),
  setVaultUnlocked: (vaultUnlocked) => set({ vaultUnlocked }),
  setSelectedCategoryId: (selectedCategoryId) => set({ selectedCategoryId }),
}))
