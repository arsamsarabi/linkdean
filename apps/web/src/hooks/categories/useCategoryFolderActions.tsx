import { useState, type ReactElement } from 'react'
import { Button, Group, Modal, Stack, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { CategoryIconPicker } from '@/components/category/CategoryIconPicker'
import { ConfirmModal } from '@/components/confirm-modal'
import { ShareCategoryDrawer } from '@/components/category/ShareCategoryDrawer'
import { SocialDrawer } from '@/components/social'
import { isCategoryIconKey, type CategoryIconKey } from '@/lib'
import type { Category } from '@/schemas'
import { useDeleteCategory, useForkCategory, useUpdateCategory } from './useCategories'

type Options = {
  /** Called after a category is deleted (e.g. navigate out of the folder). */
  onDeleted?: (id: string) => void
}

export type CategoryFolderActions = {
  forkPending: boolean
  openRename: (cat: Category) => void
  openShare: (cat: Category) => void
  openSocial: (cat: Category) => void
  openDelete: (id: string) => void
  forkCategory: (cat: Category) => void
  modals: ReactElement
}

export function useCategoryFolderActions(options: Options = {}): CategoryFolderActions {
  const { onDeleted } = options
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [shareCat, setShareCat] = useState<Category | null>(null)
  const [socialCat, setSocialCat] = useState<Category | null>(null)
  const [renameCat, setRenameCat] = useState<Category | null>(null)
  const [renameValue, setRenameValue] = useState('')
  const [renameIcon, setRenameIcon] = useState<CategoryIconKey | null>(null)

  const update = useUpdateCategory()
  const remove = useDeleteCategory()
  const fork = useForkCategory()

  function openRename(cat: Category): void {
    setRenameCat(cat)
    setRenameValue(cat.name)
    setRenameIcon(isCategoryIconKey(cat.icon) ? cat.icon : null)
  }

  async function confirmRename(): Promise<void> {
    if (!renameCat) return
    const next = renameValue.trim()
    const prevIcon = isCategoryIconKey(renameCat.icon) ? renameCat.icon : null
    const nameChanged = !!next && next !== renameCat.name
    const iconChanged = renameIcon !== prevIcon
    if (!nameChanged && !iconChanged) {
      setRenameCat(null)
      return
    }
    if (!next) return
    try {
      await update.mutateAsync({
        id: renameCat.id,
        body: {
          ...(nameChanged ? { name: next } : {}),
          ...(iconChanged ? { icon: renameIcon } : {}),
        },
      })
      setRenameCat(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function confirmDelete(): Promise<void> {
    if (!deleteId) return
    try {
      await remove.mutateAsync(deleteId)
      notifications.show({ message: 'Category deleted' })
      onDeleted?.(deleteId)
      setDeleteId(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function forkCategory(cat: Category): Promise<void> {
    try {
      const res = await fork.mutateAsync({ id: cat.id })
      notifications.show({
        message: `Copied “${res.category.name}” (${res.bookmarks_copied} bookmark${res.bookmarks_copied === 1 ? '' : 's'})`,
      })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const modals = (
    <>
      <ConfirmModal
        opened={!!deleteId}
        onClose={() => setDeleteId(null)}
        onConfirm={() => void confirmDelete()}
        title="Delete category?"
        body="Bookmarks in this category keep their links but lose this folder assignment."
        confirmLabel="Delete"
        loading={remove.isPending}
      />

      <Modal
        opened={!!renameCat}
        onClose={() => setRenameCat(null)}
        title="Edit category"
        centered
      >
        <Stack gap="lg">
          <TextInput
            label="Name"
            value={renameValue}
            onChange={(e) => setRenameValue(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void confirmRename()
            }}
            data-autofocus
          />
          <CategoryIconPicker value={renameIcon} onChange={setRenameIcon} />
          <Group justify="flex-end" gap="sm">
            <Button variant="default" onClick={() => setRenameCat(null)} disabled={update.isPending}>
              Cancel
            </Button>
            <Button
              onClick={() => void confirmRename()}
              loading={update.isPending}
              disabled={!renameValue.trim()}
            >
              Save
            </Button>
          </Group>
        </Stack>
      </Modal>

      <ShareCategoryDrawer
        opened={!!shareCat}
        onClose={() => setShareCat(null)}
        categoryId={shareCat?.id ?? null}
        categoryName={shareCat?.name ?? ''}
      />

      {socialCat ? (
        <SocialDrawer
          opened
          onClose={() => setSocialCat(null)}
          subjectType="category"
          subjectId={socialCat.id}
          title={socialCat.name}
        />
      ) : null}
    </>
  )

  return {
    forkPending: fork.isPending,
    openRename,
    openShare: setShareCat,
    openSocial: setSocialCat,
    openDelete: setDeleteId,
    forkCategory: (cat) => {
      void forkCategory(cat)
    },
    modals,
  }
}
