import { useEffect, useState } from 'react'
import { useDisclosure } from '@mantine/hooks'
import { notifications } from '@mantine/notifications'
import type { BookmarkDraft } from '@/components'
import { useDeleteBookmark, useUpdateBookmark } from '@/hooks/bookmarks'
import { useUnlockVault, useVaults } from '@/hooks/vault'
import { canWriteCategory } from '@/lib'
import type { Bookmark, Category } from '@/schemas'
import { draftFromBookmark, emptyDraft } from '@/services'

type SearchHitLike = {
  id: string
  can_write?: boolean
}

type Input = {
  bookmarks: Bookmark[]
  searchHits: SearchHitLike[]
  folderCat: Category | null
  vaultPrompt: boolean
}

type MoveTarget = {
  categoryIds: string[]
  bookmarkIds: string[]
}

/** Create/edit drawer, single delete, vault unlock-for-search. */
export function useLibraryBookmarkEditor({ bookmarks, searchHits, folderCat, vaultPrompt }: Input) {
  const [drawerOpen, { open: openDrawer, close: closeDrawer }] = useDisclosure(false)
  const [draft, setDraft] = useState<BookmarkDraft | null>(null)
  const [unlockOpen, setUnlockOpen] = useState(false)
  const [vaultPass, setVaultPass] = useState('')
  const [unlockVaultId, setUnlockVaultId] = useState<string | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [moveTarget, setMoveTarget] = useState<MoveTarget | null>(null)

  const deleteBookmark = useDeleteBookmark()
  const updateBookmark = useUpdateBookmark()
  const vaultUnlock = useUnlockVault()
  const vaults = useVaults(unlockOpen)
  const lockedVaults = (vaults.data?.vaults ?? []).filter((v) => !v.unlocked)

  const canCreateInContext = !folderCat || canWriteCategory(folderCat)

  useEffect(() => {
    if (!vaultPrompt) return
    // oxlint-disable-next-line react/set-state-in-effect -- open modal from search vault_prompt flag
    setUnlockOpen(true)
  }, [vaultPrompt])

  function openCreate(): void {
    if (!canCreateInContext) return
    if (folderCat && canWriteCategory(folderCat)) {
      setDraft({ ...emptyDraft(), category_ids: [folderCat.id] })
    } else {
      setDraft(null)
    }
    openDrawer()
  }

  function openEdit(id: string): void {
    const fromList = bookmarks.find((x) => x.id === id)
    const fromSearch = searchHits.find((x) => x.id === id)
    const b = fromList || fromSearch
    if (!b) {
      notifications.show({ color: 'red', message: 'Bookmark not found' })
      return
    }
    if ('can_write' in b && b.can_write === false) {
      notifications.show({ color: 'yellow', message: 'Read-only shared bookmark' })
      return
    }
    if (fromList) {
      setDraft(draftFromBookmark(fromList))
      openDrawer()
      return
    }
    notifications.show({ color: 'red', message: 'Open the bookmark from the library to edit' })
  }

  function onFlags(id: string, patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>): void {
    updateBookmark.mutate(
      { id, body: patch },
      {
        onError: (e) => notifications.show({ color: 'red', message: String(e) }),
      }
    )
  }

  async function unlockForSearch(): Promise<void> {
    if (!unlockVaultId) return
    try {
      await vaultUnlock.mutateAsync({ id: unlockVaultId, passphrase: vaultPass })
      setVaultPass('')
      setUnlockVaultId(null)
      setUnlockOpen(false)
      notifications.show({ message: 'Vault unlocked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function confirmDelete(): Promise<void> {
    if (!deleteId) return
    try {
      await deleteBookmark.mutateAsync(deleteId)
      notifications.show({ message: 'Bookmark deleted' })
      setDeleteId(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return {
    drawerOpen,
    closeDrawer,
    draft,
    unlockOpen,
    setUnlockOpen,
    vaultPass,
    setVaultPass,
    unlockVaultId,
    setUnlockVaultId,
    lockedVaults,
    moveTarget,
    setMoveTarget,
    deleteId,
    setDeleteId,
    deleteBookmark,
    vaultUnlock,
    canCreateInContext,
    openCreate,
    openEdit,
    onFlags,
    unlockForSearch,
    confirmDelete,
  }
}
