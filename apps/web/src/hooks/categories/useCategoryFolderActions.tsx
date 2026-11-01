import { useState, type ReactElement } from 'react'
import { notifications } from '@mantine/notifications'
import { ConfirmModal } from '@/components/confirm-modal'
import { EditCategoryDrawer } from '@/components/category/EditCategoryDrawer'
import { ShareCategoryDrawer } from '@/components/category/ShareCategoryDrawer'
import { SocialDrawer } from '@/components/social'
import type { Category } from '@/schemas'
import { useDeleteCategory, useForkCategory } from './useCategories'

type Options = {
  /** Called after a category is deleted (e.g. navigate out of the folder). */
  onDeleted?: (id: string) => void
}

export type CategoryFolderActions = {
  forkPending: boolean
  openEdit: (cat: Category) => void
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
  const [editCat, setEditCat] = useState<Category | null>(null)

  const remove = useDeleteCategory()
  const fork = useForkCategory()

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

      <EditCategoryDrawer
        opened={!!editCat}
        onClose={() => setEditCat(null)}
        category={editCat}
      />

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
    openEdit: setEditCat,
    openShare: setShareCat,
    openSocial: setSocialCat,
    openDelete: setDeleteId,
    forkCategory: (cat) => {
      void forkCategory(cat)
    },
    modals,
  }
}
