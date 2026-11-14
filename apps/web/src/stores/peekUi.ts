import { create } from 'zustand'

const PEEK_MODE_KEY = 'linkdean-peek-mode'

export type PeekMode = 'open' | 'maximized' | 'minimized'

export type PeekTarget = {
  bookmarkId: string
  url: string
  title: string
  description?: string | null
  previewUrl?: string | null
}

function readMode(): PeekMode {
  try {
    const v = localStorage.getItem(PEEK_MODE_KEY)
    if (v === 'open' || v === 'maximized' || v === 'minimized') return v
  } catch {
    // Storage blocked: start open.
  }
  return 'open'
}

function writeMode(mode: PeekMode): void {
  try {
    localStorage.setItem(PEEK_MODE_KEY, mode)
  } catch {
    // Storage blocked: the choice lasts until reload.
  }
}

type PeekUiState = {
  target: PeekTarget | null
  mode: PeekMode
  open: (target: PeekTarget) => void
  close: () => void
  setMode: (mode: PeekMode) => void
  maximize: () => void
  minimize: () => void
  restore: () => void
}

export const usePeekUi = create<PeekUiState>((set, get) => ({
  target: null,
  mode: readMode(),
  open: (target) => {
    const mode = get().mode === 'minimized' ? 'open' : get().mode
    writeMode(mode)
    set({ target, mode })
  },
  close: () => set({ target: null }),
  setMode: (mode) => {
    writeMode(mode)
    set({ mode })
  },
  maximize: () => {
    writeMode('maximized')
    set({ mode: 'maximized' })
  },
  minimize: () => {
    writeMode('minimized')
    set({ mode: 'minimized' })
  },
  restore: () => {
    writeMode('open')
    set({ mode: 'open' })
  },
}))

/** Vault bookmarks stay out of peek. Only https can load under CSP frame-src. */
export function canPeekBookmark(b: { is_vault?: boolean; url: string }): boolean {
  if (b.is_vault) return false
  try {
    return new URL(b.url).protocol === 'https:'
  } catch {
    return false
  }
}
