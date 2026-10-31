import { useEffect, useState, type ReactElement } from 'react'
import {
  Box,
  Button,
  Drawer,
  LoadingOverlay,
  MultiSelect,
  SegmentedControl,
  Stack,
  TagsInput,
  Text,
} from '@mantine/core'
import { categorySelectData, writableCategories } from '@/lib'
import type { Category } from '@/schemas'

export type BulkEditMode = 'move' | 'tag'

type Props = {
  opened: boolean
  onClose: () => void
  mode: BulkEditMode
  count: number
  categories: Category[]
  pending?: boolean
  onApplyMove: (categoryIds: string[]) => void
  onApplyTag: (tags: string[], tagMode: 'add' | 'replace') => void
}

export function BulkEditDrawer({
  opened,
  onClose,
  mode,
  count,
  categories,
  pending,
  onApplyMove,
  onApplyTag,
}: Props): ReactElement {
  const [categoryIds, setCategoryIds] = useState<string[]>([])
  const [tags, setTags] = useState<string[]>([])
  const [tagMode, setTagMode] = useState<'add' | 'replace'>('add')

  useEffect(() => {
    if (!opened) return
    setCategoryIds([])
    setTags([])
    setTagMode('add')
  }, [opened, mode])

  const writable = writableCategories(categories)
  const title = mode === 'move' ? `Move ${count} bookmark${count === 1 ? '' : 's'}` : `Tag ${count} bookmark${count === 1 ? '' : 's'}`

  return (
    <Drawer opened={opened} onClose={onClose} title={title} position="right" size="md" padding="lg">
      <Box pos="relative">
        <LoadingOverlay visible={!!pending} zIndex={100} overlayProps={{ blur: 1 }} />
        <Stack gap="md">
          {mode === 'move' ? (
            <>
              <Text size="sm" c="dimmed">
                Replaces categories on each selected bookmark. Read-only shared folders omitted.
              </Text>
              <MultiSelect
                label="Categories"
                data={categorySelectData(writable)}
                value={categoryIds}
                onChange={setCategoryIds}
                searchable
                clearable
                placeholder="Pick folders…"
              />
              <Button
                onClick={() => onApplyMove(categoryIds)}
                disabled={pending}
              >
                Apply move
              </Button>
            </>
          ) : (
            <>
              <SegmentedControl
                fullWidth
                value={tagMode}
                onChange={(v) => setTagMode(v as 'add' | 'replace')}
                data={[
                  { value: 'add', label: 'Add tags' },
                  { value: 'replace', label: 'Replace tags' },
                ]}
              />
              <TagsInput
                label="Tags"
                description={
                  tagMode === 'add'
                    ? 'Merged onto each bookmark’s existing tags.'
                    : 'Replaces all tags on each selected bookmark.'
                }
                value={tags}
                onChange={setTags}
                clearable
              />
              <Button
                onClick={() => onApplyTag(tags, tagMode)}
                disabled={pending || (tagMode === 'add' && tags.every((t) => !t.trim()))}
              >
                Apply tags
              </Button>
            </>
          )}
        </Stack>
      </Box>
    </Drawer>
  )
}
