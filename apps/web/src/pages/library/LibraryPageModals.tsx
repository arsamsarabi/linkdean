import { type ReactElement } from 'react'
import { Button, Group, Modal, PasswordInput, Select, Stack, Text } from '@mantine/core'
import { IconLockOpen } from '@tabler/icons-react'
import { BookmarkDrawer, BulkEditDrawer, ConfirmModal, MoveToVaultModal } from '@/components'
import { categoriesForSelection } from '@/services'
import { useLibraryPageModel } from './useLibraryPageModel'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LibraryPageModals(): ReactElement {
  const lib = useLibraryPageModel()
  const categories = lib.cats.data || []
  const permanent = lib.me.data?.settings.vault_delete_mode === 'permanent'
  // Look the item up everywhere it can come from: the grid, Explorer and search all open this.
  const deleting = [...lib.list, ...(lib.bookmarks.data ?? []), ...lib.searchItems].find((b) => b.id === lib.deleteId)
  const deletesVaultItem = permanent && !!deleting?.is_vault
  const bulkDeletesVaultItem = permanent && lib.selectedItems.some((b) => b.is_vault)

  return (
    <>
      <BookmarkDrawer
        vaultRequired={lib.scope === 'vault'}
        opened={lib.drawerOpen}
        onClose={lib.closeDrawer}
        categories={categories}
        initial={lib.draft}
      />

      <BulkEditDrawer
        opened={lib.bulkMode != null}
        onClose={() => lib.setBulkMode(null)}
        mode={lib.bulkMode ?? 'categories'}
        count={lib.selectedCount}
        categories={lib.scope === 'vault' ? categoriesForSelection(categories, lib.list, lib.selectedIds) : categories}
        pending={lib.bulkPending}
        onApplyCategories={(ids) => void lib.applyBulkCategories(ids)}
        onApplyTag={(tags, mode) => void lib.applyBulkTag(tags, mode)}
      />

      {lib.categoryActions.modals}

      <ConfirmModal
        opened={!!lib.deleteId}
        onClose={() => lib.setDeleteId(null)}
        onConfirm={() => void lib.confirmDelete()}
        title="Delete bookmark?"
        body={
          deletesVaultItem
            ? 'This permanently removes the bookmark. It cannot be undone.'
            : 'Moves this bookmark to Deleted items. You can restore it later.'
        }
        confirmLabel="Delete"
        loading={lib.deleteBookmark.isPending}
      />

      <ConfirmModal
        opened={lib.bulkDeleteOpen}
        onClose={() => lib.setBulkDeleteOpen(false)}
        onConfirm={() => void lib.confirmBulkDelete()}
        title={`Delete ${lib.selectedCount} bookmark${lib.selectedCount === 1 ? '' : 's'}?`}
        body={
          bulkDeletesVaultItem
            ? 'Vault bookmarks are removed permanently and cannot be recovered. Others move to Deleted items.'
            : 'Moves selected bookmarks to Deleted items. You can restore them later.'
        }
        confirmLabel="Delete"
        loading={lib.bulkDelete.isPending}
      />

      <Modal opened={lib.unlockOpen} onClose={() => lib.setUnlockOpen(false)} title="Unlock vault for search">
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            Config options say vault should be included in search. Unlock to include vault hits.
          </Text>
          <Select
            label="Vault"
            data={lib.lockedVaults.map((v) => ({ value: v.id, label: v.name }))}
            value={lib.unlockVaultId}
            onChange={lib.setUnlockVaultId}
          />
          <PasswordInput
            label="Vault passphrase"
            value={lib.vaultPass}
            onChange={(e) => lib.setVaultPass(e.currentTarget.value)}
          />
          <Group justify="flex-end">
            <Button variant="default" onClick={() => lib.setUnlockOpen(false)}>
              Skip
            </Button>
            <Button
              leftSection={<IconLockOpen {...iconSm} />}
              onClick={() => void lib.unlockForSearch()}
              disabled={!lib.unlockVaultId || lib.vaultPass.length < 8}
              loading={lib.vaultUnlock.isPending}
            >
              Unlock
            </Button>
          </Group>
        </Stack>
      </Modal>

      {lib.moveTarget ? (
        <MoveToVaultModal
          categoryIds={lib.moveTarget.categoryIds}
          bookmarkIds={lib.moveTarget.bookmarkIds}
          onClose={() => lib.setMoveTarget(null)}
        />
      ) : null}
    </>
  )
}
