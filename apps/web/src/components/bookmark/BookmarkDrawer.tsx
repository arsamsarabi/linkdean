import { useEffect, useState, type ReactElement } from 'react'
import {
  Box,
  Button,
  Checkbox,
  Drawer,
  LoadingOverlay,
  MultiSelect,
  Stack,
  TagsInput,
  Textarea,
  TextInput,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useCreateBookmark, useDeleteBookmarkPreview, useUpdateBookmark, useUploadBookmarkPreview } from '@/hooks'
import { categorySelectData, writableCategories } from '@/lib'
import type { Category } from '@/schemas'
import { emptyDraft, type BookmarkDraft } from '@/services'
import { BookmarkPreviewField } from './BookmarkPreviewField'

export type { BookmarkDraft }
export { draftFromBookmark } from '@/services'

type Props = {
  opened: boolean
  onClose: () => void
  categories: Category[]
  initial: BookmarkDraft | null
}

export function BookmarkDrawer({ opened, onClose, categories, initial }: Props): ReactElement {
  const [draft, setDraft] = useState<BookmarkDraft>(emptyDraft())
  const [pendingPreview, setPendingPreview] = useState<Blob | null>(null)
  const editing = !!draft.id
  const createBookmark = useCreateBookmark()
  const updateBookmark = useUpdateBookmark()
  const uploadPreview = useUploadBookmarkPreview()
  const deletePreview = useDeleteBookmarkPreview()
  const pending =
    createBookmark.isPending || updateBookmark.isPending || uploadPreview.isPending || deletePreview.isPending

  useEffect(() => {
    if (!opened) return
    setDraft(initial ? { ...initial, tags: [...initial.tags] } : emptyDraft())
    setPendingPreview(null)
  }, [opened, initial])

  async function save(): Promise<void> {
    const tags = draft.tags.map((t) => t.trim()).filter(Boolean)
    const body = {
      url: draft.url.trim(),
      title: draft.title.trim() || undefined,
      description: draft.description.trim() || undefined,
      category_ids: draft.category_ids,
      tags,
    }
    try {
      if (draft.id) {
        await updateBookmark.mutateAsync({
          id: draft.id,
          body: {
            url: body.url,
            title: body.title || '',
            description: body.description ?? null,
            category_ids: body.category_ids,
            tags: body.tags,
            is_favorite: draft.is_favorite,
            is_unread: draft.is_unread,
            is_pinned: draft.is_pinned,
          },
        })
        if (pendingPreview) {
          await uploadPreview.mutateAsync({ id: draft.id, blob: pendingPreview })
        }
      } else {
        const created = await createBookmark.mutateAsync({
          url: body.url,
          title: body.title,
          description: body.description,
          category_ids: body.category_ids,
          tags: body.tags,
          is_favorite: draft.is_favorite,
          is_unread: draft.is_unread,
          is_pinned: draft.is_pinned,
        })
        if (pendingPreview && !created.is_vault) {
          try {
            await uploadPreview.mutateAsync({ id: created.id, blob: pendingPreview })
          } catch (e) {
            notifications.show({
              color: 'yellow',
              message: `Bookmark saved; preview upload failed: ${e}`,
            })
            onClose()
            return
          }
        }
      }
      notifications.show({ message: editing ? 'Updated' : 'Saved' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const previewAllowed = !draft.is_vault && (draft.can_write !== false || !editing)

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={editing ? 'Edit bookmark' : 'Add bookmark'}
      position="right"
      size="md"
      padding="lg"
    >
      <Box pos="relative">
        <LoadingOverlay visible={pending} zIndex={100} overlayProps={{ blur: 1 }} />
        <Stack gap="md">
          <TextInput
            label="URL"
            required
            placeholder="https://"
            value={draft.url}
            onChange={(e) => {
              const url = e.currentTarget.value
              setDraft((d) => ({ ...d, url }))
            }}
          />
          <TextInput
            label="Title"
            value={draft.title}
            onChange={(e) => {
              const title = e.currentTarget.value
              setDraft((d) => ({ ...d, title }))
            }}
          />
          <Textarea
            label="Description"
            minRows={2}
            autosize
            value={draft.description}
            onChange={(e) => {
              const description = e.currentTarget.value
              setDraft((d) => ({ ...d, description }))
            }}
          />
          {previewAllowed ? (
            <BookmarkPreviewField
              bookmarkId={draft.id}
              hasPreview={draft.has_preview}
              previewImageUrl={draft.preview_image_url}
              updatedAt={draft.updated_at}
              pendingBlob={pendingPreview}
              onPendingChange={setPendingPreview}
              onUploaded={(next) =>
                setDraft((d) => ({
                  ...d,
                  has_preview: next.has_preview,
                  preview_image_url: next.preview_image_url,
                  updated_at: next.updated_at,
                }))
              }
              onUpload={async (id, blob) => uploadPreview.mutateAsync({ id, blob })}
              onDelete={async (id) => deletePreview.mutateAsync(id)}
              disabled={pending}
            />
          ) : null}
          <MultiSelect
            label="Categories"
            description="Can pick several. Nested path shown in label. Read-only shared folders are omitted."
            data={categorySelectData(writableCategories(categories))}
            value={draft.category_ids}
            onChange={(category_ids) => setDraft((d) => ({ ...d, category_ids }))}
            searchable
            clearable
          />
          <TagsInput
            label="Tags"
            description="Type a tag and press Enter"
            value={draft.tags}
            onChange={(tags) => setDraft((d) => ({ ...d, tags }))}
            clearable
          />
          <Checkbox
            label="Reading list (unread)"
            checked={draft.is_unread}
            onChange={(e) => {
              const is_unread = e.currentTarget.checked
              setDraft((d) => ({ ...d, is_unread }))
            }}
          />
          <Checkbox
            label="Favorite"
            checked={draft.is_favorite}
            onChange={(e) => {
              const is_favorite = e.currentTarget.checked
              setDraft((d) => ({ ...d, is_favorite }))
            }}
          />
          <Checkbox
            label="Pin to top of library"
            checked={draft.is_pinned}
            onChange={(e) => {
              const is_pinned = e.currentTarget.checked
              setDraft((d) => ({ ...d, is_pinned }))
            }}
          />
          <Button mt="sm" onClick={() => void save()} disabled={!draft.url.trim()} loading={pending}>
            {editing ? 'Save changes' : 'Add bookmark'}
          </Button>
        </Stack>
      </Box>
    </Drawer>
  )
}
