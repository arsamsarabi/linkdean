import { type ReactElement, type ReactNode } from 'react'
import { Button, Group, Modal, PasswordInput, Stack, Text } from '@mantine/core'
import { IconLockOpen } from '@tabler/icons-react'
import {
  BookmarkDrawer,
  BulkEditDrawer,
  ConfirmModal,
  DuplicateMergeDrawer,
  type BookmarkDraft,
  type BulkEditMode,
} from '@/components'
import type { Category } from '@/schemas'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  categories: Category[]
  drawerOpen: boolean
  onCloseDrawer: () => void
  draft: BookmarkDraft | null
  bulkMode: BulkEditMode | null
  onCloseBulk: () => void
  selectedCount: number
  bulkPending: boolean
  onApplyCategories: (ids: string[]) => void
  onApplyTag: (tags: string[], mode: 'add' | 'replace') => void
  dupesOpen: boolean
  onCloseDupes: () => void
  categoryModals: ReactNode
  deleteId: string | null
  onCloseDelete: () => void
  onConfirmDelete: () => void
  deletePending: boolean
  bulkDeleteOpen: boolean
  onCloseBulkDelete: () => void
  onConfirmBulkDelete: () => void
  bulkDeletePending: boolean
  unlockOpen: boolean
  onCloseUnlock: () => void
  vaultPass: string
  onVaultPassChange: (value: string) => void
  onUnlock: () => void
  unlockPending: boolean
}

export function LibraryPageModals({
  categories,
  drawerOpen,
  onCloseDrawer,
  draft,
  bulkMode,
  onCloseBulk,
  selectedCount,
  bulkPending,
  onApplyCategories,
  onApplyTag,
  dupesOpen,
  onCloseDupes,
  categoryModals,
  deleteId,
  onCloseDelete,
  onConfirmDelete,
  deletePending,
  bulkDeleteOpen,
  onCloseBulkDelete,
  onConfirmBulkDelete,
  bulkDeletePending,
  unlockOpen,
  onCloseUnlock,
  vaultPass,
  onVaultPassChange,
  onUnlock,
  unlockPending,
}: Props): ReactElement {
  return (
    <>
      <BookmarkDrawer
        opened={drawerOpen}
        onClose={onCloseDrawer}
        categories={categories}
        initial={draft}
      />

      <BulkEditDrawer
        opened={bulkMode != null}
        onClose={onCloseBulk}
        mode={bulkMode ?? 'categories'}
        count={selectedCount}
        categories={categories}
        pending={bulkPending}
        onApplyCategories={onApplyCategories}
        onApplyTag={onApplyTag}
      />

      <DuplicateMergeDrawer opened={dupesOpen} onClose={onCloseDupes} />

      {categoryModals}

      <ConfirmModal
        opened={!!deleteId}
        onClose={onCloseDelete}
        onConfirm={onConfirmDelete}
        title="Delete bookmark?"
        body="Moves this bookmark to Deleted items. You can restore it later."
        confirmLabel="Delete"
        loading={deletePending}
      />

      <ConfirmModal
        opened={bulkDeleteOpen}
        onClose={onCloseBulkDelete}
        onConfirm={onConfirmBulkDelete}
        title={`Delete ${selectedCount} bookmark${selectedCount === 1 ? '' : 's'}?`}
        body="Moves selected bookmarks to Deleted items. You can restore them later."
        confirmLabel="Delete"
        loading={bulkDeletePending}
      />

      <Modal opened={unlockOpen} onClose={onCloseUnlock} title="Unlock vault for search">
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            Config options say vault should be included in search. Unlock to include vault hits.
          </Text>
          <PasswordInput
            label="Vault passphrase"
            value={vaultPass}
            onChange={(e) => onVaultPassChange(e.currentTarget.value)}
          />
          <Group justify="flex-end">
            <Button variant="default" onClick={onCloseUnlock}>
              Skip
            </Button>
            <Button
              leftSection={<IconLockOpen {...iconSm} />}
              onClick={onUnlock}
              disabled={vaultPass.length < 8}
              loading={unlockPending}
            >
              Unlock
            </Button>
          </Group>
        </Stack>
      </Modal>
    </>
  )
}
