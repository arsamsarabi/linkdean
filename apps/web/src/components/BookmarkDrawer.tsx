import { useEffect, useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
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
import { api, type Bookmark, type Category } from '../api'
import { categorySelectData } from '../lib/categories'

export type BookmarkDraft = {
  id?: string
  url: string
  title: string
  description: string
  tags: string[]
  category_ids: string[]
}

const emptyDraft = (): BookmarkDraft => ({
  url: '',
  title: '',
  description: '',
  tags: [],
  category_ids: [],
})

export function draftFromBookmark(b: Bookmark): BookmarkDraft {
  return {
    id: b.id,
    url: b.url,
    title: b.title || '',
    description: b.description || '',
    tags: b.tags || [],
    category_ids: (b.categories || []).map((c) => c.id),
  }
}

export function BookmarkDrawer({
  opened,
  onClose,
  categories,
  initial,
}: {
  opened: boolean
  onClose: () => void
  categories: Category[]
  initial: BookmarkDraft | null
}) {
  const qc = useQueryClient()
  const [draft, setDraft] = useState<BookmarkDraft>(emptyDraft())
  const editing = !!draft.id

  useEffect(() => {
    if (opened) setDraft(initial ? { ...initial, tags: [...initial.tags] } : emptyDraft())
  }, [opened, initial])

  const save = useMutation({
    mutationFn: async () => {
      const tags = draft.tags.map((t) => t.trim()).filter(Boolean)
      const body = {
        url: draft.url.trim(),
        title: draft.title.trim() || undefined,
        description: draft.description.trim() || undefined,
        category_ids: draft.category_ids,
        tags,
      }
      if (draft.id) {
        return api.updateBookmark(draft.id, {
          url: body.url,
          title: body.title || '',
          description: body.description ?? null,
          category_ids: body.category_ids,
          tags: body.tags,
        })
      }
      return api.createBookmark({
        url: body.url,
        title: body.title,
        description: body.description,
        category_ids: body.category_ids,
        tags: body.tags,
      })
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['bookmarks'] })
      qc.invalidateQueries({ queryKey: ['search'] })
      window.setTimeout(() => qc.invalidateQueries({ queryKey: ['bookmarks'] }), 2500)
      window.setTimeout(() => qc.invalidateQueries({ queryKey: ['bookmarks'] }), 6000)
      notifications.show({ message: editing ? 'Updated' : 'Saved' })
      onClose()
    },
    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
  })

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
        <LoadingOverlay visible={save.isPending} zIndex={100} overlayProps={{ blur: 1 }} />
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
            onClick={() => save.mutate()}
            disabled={!draft.url.trim()}
            loading={save.isPending}
          >
            {editing ? 'Save changes' : 'Add bookmark'}
          </Button>
        </Stack>
      </Box>
    </Drawer>
  )
}
