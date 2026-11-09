import { useState, type ReactElement } from 'react'
import { Alert, Button, Center, Loader, Stack, Text } from '@mantine/core'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { BookmarkDrawer } from '@/components'
import { useBookmarkByUrl, useCategories, useSyncDuringRender } from '@/hooks'
import { lastSaveFolder, rememberSaveFolder, withWebScheme, writableCategories } from '@/lib'
import type { Category } from '@/schemas'
import { draftFromBookmark, draftFromShare, isFetchableUrl, type BookmarkDraft, type SharedPage } from '@/services'

export type SaveSearch = SharedPage & {
  /** Opened by the bookmarklet in its own small window: close it when done. */
  popup?: boolean
}

/** The last folder used, if it still exists and can take a new bookmark outside a vault. */
function startingFolder(categories: Category[]): string[] {
  const id = lastSaveFolder()
  return id && writableCategories(categories).some((c) => c.id === id && !c.is_vault) ? [id] : []
}

/**
 * Quick save: `/save?url=…&title=…&text=…` opens the bookmark form filled in from what the
 * share sheet, bookmarklet or link handed over. A URL already in the library opens that
 * bookmark instead of saving a second copy.
 */
export function SavePage(): ReactElement {
  const search: SaveSearch = useSearch({ from: '/_app/save' })
  const nav = useNavigate()
  const [shared] = useState(() => draftFromShare(search))
  const url = withWebScheme(shared.url)
  const fetchable = isFetchableUrl(url)
  const categories = useCategories()
  const existing = useBookmarkByUrl(fetchable ? url : null)
  // A failed lookup must not block saving: treat it as a new bookmark.
  const ready = fetchable && !categories.isPending && !existing.isPending
  const found = existing.data ?? null
  const [initial, setInitial] = useState<BookmarkDraft | null>(null)

  useSyncDuringRender([ready, found?.id], () => {
    if (!ready) return
    setInitial(
      found ? draftFromBookmark(found) : { ...shared, url, category_ids: startingFolder(categories.data ?? []) }
    )
  })

  const close = (): void => {
    // A window the bookmarklet opened can close itself; if the browser refuses, carry on in it.
    if (search.popup) window.close()
    void nav({ to: '/library' })
  }

  if (!fetchable) {
    return (
      <Stack maw={480} mx="auto" mt="xl" px="md">
        <Alert color="yellow" title="Nothing to save">
          This link did not include a web address. Quick save needs one that starts with http:// or https://.
        </Alert>
        <Button variant="default" onClick={close}>
          Go to library
        </Button>
      </Stack>
    )
  }

  if (!initial) {
    return (
      <Center mih="50vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  return (
    <BookmarkDrawer
      opened
      onClose={close}
      categories={categories.data ?? []}
      initial={initial}
      autoImport={!found}
      quickSave={!found}
      onSaved={(draft, unsorted) => {
        if (!found && !unsorted) rememberSaveFolder(draft.category_ids[0] ?? null)
      }}
      notice={
        found ? (
          <Alert color="blue" title="Already in your library">
            <Text size="sm">You saved this page before. Change it here, or close to leave it as it is.</Text>
          </Alert>
        ) : null
      }
    />
  )
}
