import { useState, type ReactElement } from 'react'
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

export type BulkEditMode = 'categories' | 'tag'

type Props = {
  opened: boolean
  onClose: () => void
  mode: BulkEditMode
  count: number
  categories: Category[]
  pending?: boolean
  onApplyCategories: (categoryIds: string[]) => void
  onApplyTag: (tags: string[], tagMode: 'add' | 'replace') => void
}

export function BulkEditDrawer({
  opened,
  onClose,
  mode,
  count,
  categories,
  pending,
  onApplyCategories,
  onApplyTag,
}: Props): ReactElement {
  const [categoryIds, setCategoryIds] = useState<string[]>([])
  const [tags, setTags] = useState<string[]>([])
  const [tagMode, setTagMode] = useState<'add' | 'replace'>('add')
  const [openEpoch, setOpenEpoch] = useState(opened)
  const [modeEpoch, setModeEpoch] = useState(mode)
  if (opened !== openEpoch || mode !== modeEpoch) {
    setOpenEpoch(opened)
    setModeEpoch(mode)
    if (opened) {
      setCategoryIds([])
      setTags([])
      setTagMode('add')
    }
  }

  const writable = writableCategories(categories)
  const title =
    mode === 'categories'
      ? `Add categories to ${count} bookmark${count === 1 ? '' : 's'}`
      : `Tag ${count} bookmark${count === 1 ? '' : 's'}`

  return (
    <Drawer opened={opened} onClose={onClose} title={title} position="right" size="md" padding="lg">
      <Box pos="relative">
        <LoadingOverlay visible={!!pending} zIndex={100} overlayProps={{ blur: 1 }} />
        <Stack gap="md">
          {mode === 'categories' ? (
            <>
              <Text size="sm" c="dimmed">
                Merges folders onto each selected bookmark. Existing categories stay. Read-only shared folders omitted.
              </Text>
              <MultiSelect
                label="Categories"
                data={categorySelectData(writable)}
                value={categoryIds}
                onChange={setCategoryIds}
                searchable
                clearable
                placeholder="Pick folders to add…"
              />
              <Button onClick={() => onApplyCategories(categoryIds)} disabled={pending || categoryIds.length === 0}>
                Add categories
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
