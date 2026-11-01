import { type ReactElement } from 'react'
import { Button, Group, Modal, PasswordInput, Stack, Text } from '@mantine/core'
import { IconLockOpen } from '@tabler/icons-react'
import { BookmarkDrawer, BulkEditDrawer, ConfirmModal, DuplicateMergeDrawer } from '@/components'
import { useLibraryPageModel } from './LibraryPageContext'

const iconSm = { size: 16, stroke: 1.5 } as const

export function LibraryPageModals(): ReactElement {
  const lib = useLibraryPageModel()
  const categories = lib.cats.data || []

  return (
    <>
      <BookmarkDrawer opened={lib.drawerOpen} onClose={lib.closeDrawer} categories={categories} initial={lib.draft} />

      <BulkEditDrawer
        opened={lib.bulkMode != null}
        onClose={() => lib.setBulkMode(null)}
        mode={lib.bulkMode ?? 'categories'}
        count={lib.selectedCount}
        categories={categories}
        pending={lib.bulkPending}
        onApplyCategories={(ids) => void lib.applyBulkCategories(ids)}
        onApplyTag={(tags, mode) => void lib.applyBulkTag(tags, mode)}
      />

      <DuplicateMergeDrawer opened={lib.dupesOpen} onClose={lib.closeDupes} />

      {lib.categoryActions.modals}

      <ConfirmModal
        opened={!!lib.deleteId}
        onClose={() => lib.setDeleteId(null)}
        onConfirm={() => void lib.confirmDelete()}
        title="Delete bookmark?"
        body="Moves this bookmark to Deleted items. You can restore it later."
        confirmLabel="Delete"
        loading={lib.deleteBookmark.isPending}
      />

      <ConfirmModal
        opened={lib.bulkDeleteOpen}
        onClose={() => lib.setBulkDeleteOpen(false)}
        onConfirm={() => void lib.confirmBulkDelete()}
        title={`Delete ${lib.selectedCount} bookmark${lib.selectedCount === 1 ? '' : 's'}?`}
        body="Moves selected bookmarks to Deleted items. You can restore them later."
        confirmLabel="Delete"
        loading={lib.bulkDelete.isPending}
      />

      <Modal opened={lib.unlockOpen} onClose={() => lib.setUnlockOpen(false)} title="Unlock vault for search">
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            Config options say vault should be included in search. Unlock to include vault hits.
          </Text>
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
              disabled={lib.vaultPass.length < 8}
              loading={lib.vaultUnlock.isPending}
            >
              Unlock
            </Button>
          </Group>
        </Stack>
      </Modal>
    </>
  )
}
