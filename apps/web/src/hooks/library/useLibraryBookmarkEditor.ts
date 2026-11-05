import { useEffect, useState } from 'react'
import { useDisclosure } from '@mantine/hooks'
import { notifications } from '@mantine/notifications'
import { api } from '@/api'
import type { BookmarkDraft } from '@/components'
import { useDeleteBookmark, useUpdateBookmark } from '@/hooks/bookmarks'
import { useUnlockVault, useVaults } from '@/hooks/vault'
import { canWriteCategory } from '@/lib'
import type { Bookmark, Category } from '@/schemas'
import { draftFromBookmark, emptyDraft } from '@/services'
import type { LibraryScope } from './types'

type SearchHitLike = {
  id: string
  can_write?: boolean
}

type Input = {
  bookmarks: Bookmark[]
  searchHits: SearchHitLike[]
  folderCat: Category | null
  vaultPrompt: boolean
  scope: LibraryScope
  vaultId: string | null
}

const promptDismissedKey = 'linkdean_vault_search_prompt_dismissed'

function readPromptDismissed(): boolean {
  try {
    return sessionStorage.getItem(promptDismissedKey) === '1'
  } catch {
    return false
  }
}

function writePromptDismissed(): void {
  try {
    sessionStorage.setItem(promptDismissedKey, '1')
  } catch {
    /* storage blocked: the dismissal lasts until reload */
  }
}

type MoveTarget = {
  categoryIds: string[]
  bookmarkIds: string[]
}

/** Create/edit drawer, single delete, vault unlock-for-search. */
export function useLibraryBookmarkEditor({ bookmarks, searchHits, folderCat, vaultPrompt, scope, vaultId }: Input) {
  const [drawerOpen, { open: openDrawer, close: closeDrawer }] = useDisclosure(false)
  const [draft, setDraft] = useState<BookmarkDraft | null>(null)
  const [unlockOpen, setUnlockOpen] = useState(false)
  const [promptDismissed, setPromptDismissed] = useState(readPromptDismissed)
  const [vaultPass, setVaultPass] = useState('')
  const [unlockVaultId, setUnlockVaultId] = useState<string | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [moveTarget, setMoveTarget] = useState<MoveTarget | null>(null)

  const deleteBookmark = useDeleteBookmark()
  const updateBookmark = useUpdateBookmark()
  const vaultUnlock = useUnlockVault()
  const vaults = useVaults(unlockOpen || scope === 'vault')
  const lockedVaults = (vaults.data?.vaults ?? []).filter((v) => !v.unlocked)
  const openVaults = (vaults.data?.vaults ?? []).filter((v) => v.unlocked)

  const canCreateInContext = !folderCat || canWriteCategory(folderCat)

  useEffect(() => {
    // Every search carries the prompt, so once dismissed it stays closed for this tab.
    if (!vaultPrompt || promptDismissed) return
    // oxlint-disable-next-line react/set-state-in-effect -- open modal from search vault_prompt flag
    setUnlockOpen(true)
  }, [vaultPrompt, promptDismissed])

  function dismissUnlock(): void {
    setUnlockOpen(false)
    setPromptDismissed(true)
    writePromptDismissed()
  }

  function openCreate(): void {
    openCreateWithUrl('')
  }

  /** Paste to add: the new-bookmark drawer with the URL already filled in. */
  function openCreateWithUrl(url: string): void {
    if (!canCreateInContext) return
    if (folderCat && canWriteCategory(folderCat)) {
      setDraft({ ...emptyDraft(), url, category_ids: [folderCat.id] })
    } else if (scope === 'vault') {
      setDraft({ ...emptyDraft(), url, vault_id: vaultId ?? (openVaults.length === 1 ? openVaults[0].id : null) })
    } else {
      setDraft(url ? { ...emptyDraft(), url } : null)
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
    // A search hit whose page is not loaded: the draft needs the full bookmark.
    api
      .bookmark(id)
      .then((full) => {
        setDraft(draftFromBookmark(full))
        openDrawer()
      })
      .catch((e: unknown) => notifications.show({ color: 'red', message: String(e) }))
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
    dismissUnlock,
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
    openCreateWithUrl,
    openEdit,
    onFlags,
    unlockForSearch,
    confirmDelete,
  }
}
