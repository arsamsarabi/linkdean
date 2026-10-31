import { useEffect, useState, type ReactElement } from 'react'
import {
  Box,
  Button,
  Drawer,
  LoadingOverlay,
  MultiSelect,
  Stack,
  TagsInput,
  TextInput,
  Textarea,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import type { Category } from '../../schemas'
import { useCreateBookmark, useUpdateBookmark } from '../../hooks/bookmarks/useBookmarks'
import { categorySelectData } from '../../lib/categories/categories'
import { emptyDraft, type BookmarkDraft } from '../../services/bookmarks/bookmarks'

export type { BookmarkDraft }
export { draftFromBookmark } from '../../services/bookmarks/bookmarks'

type Props = {
  opened: boolean
  onClose: () => void
  categories: Category[]
  initial: BookmarkDraft | null
}

export function BookmarkDrawer({ opened, onClose, categories, initial }: Props): ReactElement {
  const [draft, setDraft] = useState<BookmarkDraft>(emptyDraft())
  const editing = !!draft.id
  const createBookmark = useCreateBookmark()
  const updateBookmark = useUpdateBookmark()
  const pending = createBookmark.isPending || updateBookmark.isPending

  useEffect(() => {
    if (opened) setDraft(initial ? { ...initial, tags: [...initial.tags] } : emptyDraft())
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
          },
        })
      } else {
        await createBookmark.mutateAsync({
          url: body.url,
          title: body.title,
          description: body.description,
          category_ids: body.category_ids,
          tags: body.tags,
        })
      }
      notifications.show({ message: editing ? 'Updated' : 'Saved' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

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
            onChange={(e) => setDraft((d) => ({ ...d, url: e.currentTarget.value }))}
          />
          <TextInput
            label="Title"
            value={draft.title}
            onChange={(e) => setDraft((d) => ({ ...d, title: e.currentTarget.value }))}
          />
          <Textarea
            label="Description"
            minRows={2}
            autosize
            value={draft.description}
            onChange={(e) => setDraft((d) => ({ ...d, description: e.currentTarget.value }))}
          />
          <MultiSelect
            label="Categories"
            description="Can pick several. Nested path shown in label."
            data={categorySelectData(categories)}
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
          <Button
            mt="sm"
            onClick={() => void save()}
            disabled={!draft.url.trim()}
            loading={pending}
          >
            {editing ? 'Save changes' : 'Add bookmark'}
          </Button>
        </Stack>
      </Box>
    </Drawer>
  )
}
