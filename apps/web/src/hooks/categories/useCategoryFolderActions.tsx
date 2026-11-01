import { useState, type ReactElement } from 'react'
import { notifications } from '@mantine/notifications'
import { EditCategoryDrawer } from '@/components/category/EditCategoryDrawer'
import { ShareCategoryDrawer } from '@/components/category/ShareCategoryDrawer'
import { ConfirmModal } from '@/components/confirm-modal'
import { SocialDrawer } from '@/components/social'
import { MoveToVaultModal } from '@/components/vault/MoveToVaultModal'
import { useMe } from '@/hooks/auth'
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
  openDelete: (id: string, isVault?: boolean) => void
  openMove: (cat: Category) => void
  forkCategory: (cat: Category) => void
  modals: ReactElement
}

export function useCategoryFolderActions(options: Options = {}): CategoryFolderActions {
  const { onDeleted } = options
  const me = useMe()
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [deleteIsVault, setDeleteIsVault] = useState(false)
  const [shareCat, setShareCat] = useState<Category | null>(null)
  const [socialCat, setSocialCat] = useState<Category | null>(null)
  const [editCat, setEditCat] = useState<Category | null>(null)
  const [moveCat, setMoveCat] = useState<Category | null>(null)

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
        body={
          deleteIsVault && me.data?.settings.vault_delete_mode === 'permanent'
            ? 'This permanently removes the category and its subcategories. It cannot be undone. Bookmarks stay in the vault.'
            : 'Bookmarks in this category keep their links but lose this folder assignment.'
        }
        confirmLabel="Delete"
        loading={remove.isPending}
      />

      <EditCategoryDrawer opened={!!editCat} onClose={() => setEditCat(null)} category={editCat} />

      <ShareCategoryDrawer
        opened={!!shareCat}
        onClose={() => setShareCat(null)}
        categoryId={shareCat?.id ?? null}
        categoryName={shareCat?.name ?? ''}
        isVault={!!shareCat?.is_vault}
      />

      {moveCat ? (
        <MoveToVaultModal categoryIds={[moveCat.id]} bookmarkIds={[]} onClose={() => setMoveCat(null)} />
      ) : null}

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
    openDelete: (id, isVault = false) => {
      setDeleteIsVault(isVault)
      setDeleteId(id)
    },
    openMove: setMoveCat,
    forkCategory: (cat) => {
      void forkCategory(cat)
    },
    modals,
  }
}
