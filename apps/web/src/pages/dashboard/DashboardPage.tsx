import { useEffect, useMemo, useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Button,
  Group,
  MultiSelect,
  SimpleGrid,
  Stack,
  Text,
  TextInput,
  Title,
  Tooltip,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  IconDeviceFloppy,
  IconLayoutDashboard,
  IconPencil,
  IconPlus,
  IconTrash,
  IconX,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { BookmarkCard, EmptyState, QueryErrorAlert } from '@/components'
import { useBookmarks, useDefaultDashboard, usePutDefaultDashboard } from '@/hooks'
import type { Dashboard, PutDashboardLayout } from '@/schemas'

const iconSm = { size: 16, stroke: 1.5 } as const

type DraftSection = {
  key: string
  title: string
  bookmark_ids: string[]
}

type Draft = {
  title: string
  sections: DraftSection[]
}

function draftFromDashboard(d: Dashboard): Draft {
  return {
    title: d.title,
    sections: d.sections.map((s) => ({
      key: s.id,
      title: s.title,
      bookmark_ids: s.items.map((i) => i.bookmark.id),
    })),
  }
}

function emptyDraft(): Draft {
  return {
    title: 'Home',
    sections: [{ key: crypto.randomUUID(), title: 'Favourites', bookmark_ids: [] }],
  }
}

export function DashboardPage(): ReactElement {
  const dashboard = useDefaultDashboard()
  const bookmarks = useBookmarks()
  const save = usePutDefaultDashboard()
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState<Draft | null>(null)

  useEffect(() => {
    if (!editing && dashboard.data) {
      // oxlint-disable-next-line react/set-state-in-effect -- sync draft when leaving edit or first load
      setDraft(null)
    }
  }, [editing, dashboard.data])

  const bookmarkOptions = useMemo(() => {
    return (bookmarks.data || []).map((b) => ({
      value: b.id,
      label: b.title || b.url,
    }))
  }, [bookmarks.data])

  const bookmarkById = useMemo(() => {
    const map = new Map((bookmarks.data || []).map((b) => [b.id, b]))
    return map
  }, [bookmarks.data])

  function startEdit(): void {
    setDraft(dashboard.data ? draftFromDashboard(dashboard.data) : emptyDraft())
    setEditing(true)
  }

  function cancelEdit(): void {
    setEditing(false)
    setDraft(null)
  }

  async function onSave(): Promise<void> {
    if (!draft) return
    const body: PutDashboardLayout = {
      title: draft.title.trim() || 'Home',
      sections: draft.sections
        .map((s) => ({
          title: s.title.trim(),
          bookmark_ids: s.bookmark_ids,
        }))
        .filter((s) => s.title.length > 0),
    }
    try {
      await save.mutateAsync(body)
      notifications.show({ message: 'Dashboard saved' })
      setEditing(false)
      setDraft(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  if (dashboard.isError) {
    return (
      <QueryErrorAlert
        error={dashboard.error}
        onRetry={() => void dashboard.refetch()}
        title="Dashboard failed"
      />
    )
  }

  if (dashboard.isLoading || !dashboard.data) {
    return (
      <Text size="sm" c="dimmed">
        Loading dashboard…
      </Text>
    )
  }

  const view = dashboard.data
  const empty = view.sections.length === 0 || view.sections.every((s) => s.items.length === 0)

  return (
    <Stack gap="xl">
      <Group justify="space-between" align="flex-end" wrap="wrap" gap="md">
        <Stack gap={4}>
          {editing && draft ? (
            <TextInput
              aria-label="Dashboard title"
              value={draft.title}
              onChange={(e) => setDraft({ ...draft, title: e.currentTarget.value })}
              styles={{ input: { fontSize: '1.5rem', fontWeight: 700 } }}
            />
          ) : (
            <Title order={2}>{view.title}</Title>
          )}
          <Text size="sm" c="dimmed">
            Your personal home. Pin frequently used links into sections.
          </Text>
        </Stack>
        <Group gap="sm">
          {editing ? (
            <>
              <Button variant="default" leftSection={<IconX {...iconSm} />} onClick={cancelEdit}>
                Cancel
              </Button>
              <Button
                leftSection={<IconDeviceFloppy {...iconSm} />}
                onClick={() => void onSave()}
                loading={save.isPending}
              >
                Save
              </Button>
            </>
          ) : (
            <Button leftSection={<IconPencil {...iconSm} />} variant="light" onClick={startEdit}>
              Edit layout
            </Button>
          )}
        </Group>
      </Group>

      {editing && draft ? (
        <Stack gap="lg">
          {draft.sections.map((section, idx) => (
            <Stack key={section.key} gap="sm" p="md" style={{ border: '1px solid var(--mantine-color-default-border)', borderRadius: 8 }}>
              <Group justify="space-between" align="flex-end" wrap="nowrap">
                <TextInput
                  label="Section"
                  style={{ flex: 1 }}
                  value={section.title}
                  onChange={(e) => {
                    const next = [...draft.sections]
                    next[idx] = { ...section, title: e.currentTarget.value }
                    setDraft({ ...draft, sections: next })
                  }}
                />
                <Tooltip label="Remove section">
                  <ActionIcon
                    variant="subtle"
                    color="red"
                    aria-label="Remove section"
                    onClick={() =>
                      setDraft({
                        ...draft,
                        sections: draft.sections.filter((_, i) => i !== idx),
                      })
                    }
                  >
                    <IconTrash size={16} stroke={1.5} />
                  </ActionIcon>
                </Tooltip>
              </Group>
              <MultiSelect
                label="Bookmarks"
                placeholder="Pick bookmarks"
                searchable
                clearable
                data={bookmarkOptions}
                value={section.bookmark_ids}
                onChange={(bookmark_ids) => {
                  const next = [...draft.sections]
                  next[idx] = { ...section, bookmark_ids }
                  setDraft({ ...draft, sections: next })
                }}
              />
              {section.bookmark_ids.length > 0 ? (
                <SimpleGrid cols={{ base: 1, sm: 2, lg: 3, xl: 4 }} spacing="md">
                  {section.bookmark_ids.map((id) => {
                    const b = bookmarkById.get(id)
                    if (!b) return null
                    return (
                      <BookmarkCard
                        key={id}
                        bookmark={{ ...b, can_write: false }}
                        onEdit={() => undefined}
                        onDelete={() => undefined}
                      />
                    )
                  })}
                </SimpleGrid>
              ) : null}
            </Stack>
          ))}
          <Button
            variant="light"
            leftSection={<IconPlus {...iconSm} />}
            onClick={() =>
              setDraft({
                ...draft,
                sections: [
                  ...draft.sections,
                  { key: crypto.randomUUID(), title: 'New section', bookmark_ids: [] },
                ],
              })
            }
          >
            Add section
          </Button>
        </Stack>
      ) : empty ? (
        <EmptyState
          icon={<IconLayoutDashboard size={28} stroke={1.5} />}
          title="Dashboard is empty"
          description="Add sections and pick bookmarks from your library."
          action={{
            label: 'Edit layout',
            onClick: startEdit,
            leftSection: <IconPencil {...iconSm} />,
          }}
        />
      ) : (
        <Stack gap="xl">
          {view.sections.map((section) => (
            <Stack key={section.id} gap="md">
              <Title order={3}>{section.title}</Title>
              {section.items.length === 0 ? (
                <Text size="sm" c="dimmed">
                  No bookmarks in this section.
                </Text>
              ) : (
                <SimpleGrid cols={{ base: 1, sm: 2, lg: 3, xl: 4 }} spacing="lg">
                  {section.items.map((item) => (
                    <BookmarkCard
                      key={item.id}
                      bookmark={{ ...item.bookmark, can_write: false }}
                      onEdit={() => undefined}
                      onDelete={() => undefined}
                    />
                  ))}
                </SimpleGrid>
              )}
            </Stack>
          ))}
          <Text size="sm" c="dimmed">
            Browse the full collection in the{' '}
            <Text span component={Link} to="/library" c="violet" inherit>
              library
            </Text>
            .
          </Text>
        </Stack>
      )}
    </Stack>
  )
}
