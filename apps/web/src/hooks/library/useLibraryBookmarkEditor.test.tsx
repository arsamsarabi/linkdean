import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useLibraryBookmarkEditor } from './useLibraryBookmarkEditor'

vi.mock('@/hooks/bookmarks', () => ({
  useDeleteBookmark: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useUpdateBookmark: () => ({ isPending: false, mutateAsync: vi.fn() }),
}))
vi.mock('@/hooks/vault', () => ({
  useUnlockVault: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useVaults: () => ({ data: { vaults: [] } }),
}))

function useEditor(vaultPrompt: boolean) {
  return useLibraryBookmarkEditor({
    bookmarks: [],
    searchHits: [],
    folderCat: null,
    vaultPrompt,
    scope: 'library',
    vaultId: null,
  })
}

describe('vault unlock prompt', () => {
  afterEach(() => sessionStorage.clear())

  it('stays closed after a dismissal, through later searches', () => {
    const { result, rerender } = renderHook(({ prompt }) => useEditor(prompt), { initialProps: { prompt: true } })
    expect(result.current.unlockOpen).toBe(true)
    act(() => result.current.dismissUnlock())
    expect(result.current.unlockOpen).toBe(false)

    // The next search clears and sets the flag again.
    rerender({ prompt: false })
    rerender({ prompt: true })
    expect(result.current.unlockOpen).toBe(false)
  })
})
