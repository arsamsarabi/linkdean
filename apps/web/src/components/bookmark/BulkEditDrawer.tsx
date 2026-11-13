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
import { useDrawerClosesOnEscape, useDrawerSize } from '@/hooks/render'
import { categorySelectData, writableCategories } from '@/lib'
import type { Category } from '@/schemas'
import { ConfirmModal, useDiscardGuard } from '../confirm-modal'

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
  const closeOnEscape = useDrawerClosesOnEscape()
  const drawerSize = useDrawerSize()
  const [categoryIds, setCategoryIds] = useState<string[]>([])
  const [tags, setTags] = useState<string[]>([])
  const [tagMode, setTagMode] = useState<'add' | 'replace'>('add')
  const [confirmReplace, setConfirmReplace] = useState(false)
  const [openEpoch, setOpenEpoch] = useState(opened)
  const [modeEpoch, setModeEpoch] = useState(mode)
  if (opened !== openEpoch || mode !== modeEpoch) {
    setOpenEpoch(opened)
    setModeEpoch(mode)
    if (opened) {
      setCategoryIds([])
      setTags([])
      setTagMode('add')
      setConfirmReplace(false)
    }
  }
  const noTags = tags.every((t) => !t.trim())
  const { requestClose, discardModal } = useDiscardGuard(categoryIds.length > 0 || !noTags, onClose)

  const writable = writableCategories(categories)
  const title =
    mode === 'categories'
      ? `Add categories to ${count} bookmark${count === 1 ? '' : 's'}`
      : `Tag ${count} bookmark${count === 1 ? '' : 's'}`

  return (
    <>
      <Drawer
        opened={opened}
        onClose={requestClose}
        closeOnEscape={closeOnEscape}
        title={title}
        position="right"
        size={drawerSize}
        padding="lg"
      >
        <Box pos="relative">
          <LoadingOverlay visible={!!pending} zIndex={100} overlayProps={{ blur: 1 }} />
          <Stack gap="md">
            {mode === 'categories' ? (
              <>
                <Text size="sm" c="dimmed">
                  Merges folders onto each selected bookmark. Existing categories stay. Read-only shared folders
                  omitted.
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
                  onClick={() => (tagMode === 'replace' ? setConfirmReplace(true) : onApplyTag(tags, tagMode))}
                  disabled={pending || noTags}
                >
                  Apply tags
                </Button>
                <ConfirmModal
                  opened={confirmReplace}
                  onClose={() => setConfirmReplace(false)}
                  onConfirm={() => {
                    setConfirmReplace(false)
                    onApplyTag(tags, 'replace')
                  }}
                  title="Replace tags?"
                  body={`Every tag on ${count} bookmark${count === 1 ? '' : 's'} is replaced with ${tags.join(', ')}. The old tags are not kept.`}
                  confirmLabel="Replace tags"
                />
              </>
            )}
          </Stack>
        </Box>
      </Drawer>
      {discardModal}
    </>
  )
}
