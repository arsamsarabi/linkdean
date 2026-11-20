import { useState, type ReactElement } from 'react'
import { ActionIcon, Button, Group, Menu, Tooltip } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconBookmark, IconDeviceFloppy, IconPencil, IconTrash } from '@tabler/icons-react'
import { useNavigate } from '@tanstack/react-router'
import { ConfirmModal } from '@/components'
import { useCreateSavedSearch, useDeleteSavedSearch, usePatchSavedSearch, useSavedSearches } from '@/hooks'
import { filtersFromLibraryState, librarySearchFromSavedFilters, pathForSavedFilters } from '@/lib'
import type { SavedSearch } from '@/schemas'
import { SaveSearchModal } from './SaveSearchModal'
import { useLibraryPageModel } from './useLibraryPageModel'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LibrarySavedSearchControls(): ReactElement | null {
  const lib = useLibraryPageModel()
  const nav = useNavigate()
  const list = useSavedSearches()
  const create = useCreateSavedSearch()
  const patch = usePatchSavedSearch()
  const remove = useDeleteSavedSearch()

  const [saveOpen, setSaveOpen] = useState(false)
  const [renameTarget, setRenameTarget] = useState<SavedSearch | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)

  const canSave = !!lib.q.trim() || lib.filterActive
  const hasSaved = (list.data?.length ?? 0) > 0
  if (!canSave && !hasSaved && !list.isLoading) return null

  function currentFilters() {
    return filtersFromLibraryState({
      q: lib.q,
      sort: lib.sort,
      category: lib.filterCats,
      tag: lib.filterTags,
      flag: lib.filterFlags,
      since: lib.filterSince,
      vault: lib.vaultOnly,
      vaultId: lib.vaultId,
      folder: lib.folderId || 'all',
    })
  }

  function openSaved(s: SavedSearch): void {
    void nav({
      to: pathForSavedFilters(s.filters),
      search: librarySearchFromSavedFilters(s.filters),
    })
  }

  return (
    <>
      <Group gap="xs" wrap="nowrap" align="stretch" h="100%">
        {canSave ? (
          <Tooltip label="Save current search">
            <Button
              variant="default"
              leftSection={<IconDeviceFloppy {...iconSm} />}
              onClick={() => setSaveOpen(true)}
              h="100%"
            >
              Save search
            </Button>
          </Tooltip>
        ) : null}
        {hasSaved || list.isLoading ? (
          <Menu shadow="md" width={280} position="bottom-end">
            <Menu.Target>
              <Button variant="default" leftSection={<IconBookmark {...iconSm} />} h="100%">
                Saved
              </Button>
            </Menu.Target>
            <Menu.Dropdown>
              {(list.data ?? []).map((s) => (
                <Menu.Item
                  key={s.id}
                  closeMenuOnClick={false}
                  onClick={() => openSaved(s)}
                  rightSection={
                    <Group gap={2} wrap="nowrap" onClick={(e) => e.stopPropagation()}>
                      <ActionIcon
                        size="sm"
                        variant="subtle"
                        aria-label={`Rename ${s.name}`}
                        onClick={() => setRenameTarget(s)}
                      >
                        <IconPencil size={14} stroke={1.5} />
                      </ActionIcon>
                      <ActionIcon
                        size="sm"
                        variant="subtle"
                        color="red"
                        aria-label={`Delete ${s.name}`}
                        onClick={() => setDeleteId(s.id)}
                      >
                        <IconTrash size={14} stroke={1.5} />
                      </ActionIcon>
                    </Group>
                  }
                >
                  {s.name}
                </Menu.Item>
              ))}
            </Menu.Dropdown>
          </Menu>
        ) : null}
      </Group>

      <SaveSearchModal
        opened={saveOpen}
        onClose={() => setSaveOpen(false)}
        loading={create.isPending}
        onSubmit={async (name) => {
          try {
            await create.mutateAsync({ name, filters: currentFilters() })
            setSaveOpen(false)
            notifications.show({ message: 'Search saved' })
          } catch (e) {
            notifications.show({ color: 'red', message: String(e) })
          }
        }}
      />

      <SaveSearchModal
        opened={!!renameTarget}
        onClose={() => setRenameTarget(null)}
        title="Rename saved search"
        initialName={renameTarget?.name ?? ''}
        confirmLabel="Rename"
        loading={patch.isPending}
        onSubmit={async (name) => {
          if (!renameTarget) return
          try {
            await patch.mutateAsync({ id: renameTarget.id, name })
            setRenameTarget(null)
            notifications.show({ message: 'Renamed' })
          } catch (e) {
            notifications.show({ color: 'red', message: String(e) })
          }
        }}
      />

      <ConfirmModal
        opened={!!deleteId}
        onClose={() => setDeleteId(null)}
        title="Delete saved search?"
        body="This only removes the saved filter. Your bookmarks stay."
        confirmLabel="Delete"
        confirmColor="red"
        loading={remove.isPending}
        onConfirm={() => {
          if (!deleteId) return
          void remove
            .mutateAsync(deleteId)
            .then(() => {
              setDeleteId(null)
              notifications.show({ message: 'Deleted' })
            })
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
      />
    </>
  )
}
