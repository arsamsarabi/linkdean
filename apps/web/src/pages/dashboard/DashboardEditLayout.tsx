import { type ReactElement } from 'react'
import { ActionIcon, Button, Group, MultiSelect, SimpleGrid, Stack, Text, TextInput, Tooltip } from '@mantine/core'
import { IconArrowDown, IconArrowUp, IconPlus, IconTrash } from '@tabler/icons-react'
import { BookmarkCard } from '@/components'
import type { Bookmark } from '@/schemas'
import type { DashboardDraft } from '@/services'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  draft: DashboardDraft
  bookmarkOptions: { value: string; label: string }[]
  bookmarkById: Map<string, Bookmark>
  viewerUserId: string | undefined
  onUpdateTitle: (idx: number, title: string) => void
  onUpdateBookmarks: (idx: number, bookmarkIds: string[]) => void
  onMoveSection: (idx: number, delta: -1 | 1) => void
  onRemoveSection: (idx: number) => void
  onAddSection: () => void
}

export function DashboardEditLayout({
  draft,
  bookmarkOptions,
  bookmarkById,
  viewerUserId,
  onUpdateTitle,
  onUpdateBookmarks,
  onMoveSection,
  onRemoveSection,
  onAddSection,
}: Props): ReactElement {
  return (
    <Stack gap="md">
      {bookmarkOptions.length === 0 ? (
        <Text size="sm" c="orange">
          No bookmarks in your library yet. Add some in the library, then return here to pin them.
        </Text>
      ) : null}
      {draft.sections.map((section, idx) => (
        <Stack
          key={section.key}
          gap="sm"
          p="md"
          style={{ border: '1px solid var(--mantine-color-default-border)', borderRadius: 8 }}
        >
          <Group justify="space-between" align="flex-end" wrap="nowrap" gap="xs">
            <TextInput
              label="Section"
              style={{ flex: 1 }}
              value={section.title}
              onChange={(e) => onUpdateTitle(idx, e.currentTarget.value)}
            />
            <Group gap={4} wrap="nowrap">
              <Tooltip label="Move up">
                <ActionIcon
                  variant="subtle"
                  aria-label="Move section up"
                  disabled={idx === 0}
                  onClick={() => onMoveSection(idx, -1)}
                >
                  <IconArrowUp size={16} stroke={1.5} />
                </ActionIcon>
              </Tooltip>
              <Tooltip label="Move down">
                <ActionIcon
                  variant="subtle"
                  aria-label="Move section down"
                  disabled={idx === draft.sections.length - 1}
                  onClick={() => onMoveSection(idx, 1)}
                >
                  <IconArrowDown size={16} stroke={1.5} />
                </ActionIcon>
              </Tooltip>
              <Tooltip label="Remove section">
                <ActionIcon
                  variant="subtle"
                  color="red"
                  aria-label="Remove section"
                  onClick={() => onRemoveSection(idx)}
                >
                  <IconTrash size={16} stroke={1.5} />
                </ActionIcon>
              </Tooltip>
            </Group>
          </Group>
          <MultiSelect
            label="Bookmarks"
            placeholder="Pick bookmarks from your library"
            searchable
            clearable
            data={bookmarkOptions}
            value={section.bookmark_ids}
            onChange={(bookmark_ids) => onUpdateBookmarks(idx, bookmark_ids)}
          />
          {section.bookmark_ids.length === 0 ? (
            <Text size="xs" c="dimmed">
              No bookmarks yet — pick a few above.
            </Text>
          ) : (
            <SimpleGrid cols={{ base: 2, sm: 4, lg: 6, xl: 8 }} spacing="sm">
              {section.bookmark_ids.map((id) => {
                const b = bookmarkById.get(id)
                if (!b) {
                  return (
                    <Text key={id} size="xs" c="dimmed">
                      Unavailable bookmark
                    </Text>
                  )
                }
                return (
                  <BookmarkCard
                    key={id}
                    bookmark={{ ...b, can_write: false }}
                    viewerUserId={viewerUserId}
                    onEdit={() => undefined}
                    onDelete={() => undefined}
                  />
                )
              })}
            </SimpleGrid>
          )}
        </Stack>
      ))}
      {draft.sections.length === 0 ? (
        <Text size="sm" c="dimmed">
          No sections. Add one to start pinning links.
        </Text>
      ) : null}
      <Button type="button" variant="light" leftSection={<IconPlus {...iconSm} />} onClick={onAddSection}>
        Add section
      </Button>
    </Stack>
  )
}
