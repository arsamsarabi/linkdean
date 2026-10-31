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
  IconArrowDown,
  IconArrowUp,
  IconDeviceFloppy,
  IconLayoutDashboard,
  IconPencil,
  IconPlus,
  IconTrash,
  IconX,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { EmptyState, GlanceTile, QueryErrorAlert } from '@/components'
import { useBookmarks, useDefaultDashboard, useMe, usePutDefaultDashboard } from '@/hooks'
import type { Dashboard } from '@/schemas'
import {
  emptyDashboardDraft,
  moveSection,
  toPutDashboardLayout,
  type DashboardDraft,
} from '@/services'

const iconSm = { size: 16, stroke: 1.5 } as const

function draftFromDashboard(d: Dashboard): DashboardDraft {
  return {
    title: d.title,
    sections: d.sections.map((s) => ({
      key: s.id,
      title: s.title,
      bookmark_ids: s.items.map((i) => i.bookmark.id),
    })),
  }
}

export function DashboardPage(): ReactElement {
  const me = useMe()
  const dashboard = useDefaultDashboard()
  const bookmarks = useBookmarks()
  const save = usePutDefaultDashboard()
  const viewerUserId = me.data?.user.id
  const [editing, setEditing] = useState(false)
  const [draft, setDraft] = useState<DashboardDraft | null>(null)

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
    setDraft(dashboard.data ? draftFromDashboard(dashboard.data) : emptyDashboardDraft())
    setEditing(true)
  }

  function cancelEdit(): void {
    setEditing(false)
    setDraft(null)
  }

  async function onSave(): Promise<void> {
    if (!draft) return
    try {
      await save.mutateAsync(toPutDashboardLayout(draft))
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
    <Stack gap="lg">
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
            Glance at pinned links. Edit layout to rearrange sections.
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
        <Stack gap="md">
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
                  onChange={(e) => {
                    const next = [...draft.sections]
                    next[idx] = { ...section, title: e.currentTarget.value }
                    setDraft({ ...draft, sections: next })
                  }}
                />
                <Group gap={4} wrap="nowrap">
                  <Tooltip label="Move up">
                    <ActionIcon
                      variant="subtle"
                      aria-label="Move section up"
                      disabled={idx === 0}
                      onClick={() =>
                        setDraft({ ...draft, sections: moveSection(draft.sections, idx, -1) })
                      }
                    >
                      <IconArrowUp size={16} stroke={1.5} />
                    </ActionIcon>
                  </Tooltip>
                  <Tooltip label="Move down">
                    <ActionIcon
                      variant="subtle"
                      aria-label="Move section down"
                      disabled={idx === draft.sections.length - 1}
                      onClick={() =>
                        setDraft({ ...draft, sections: moveSection(draft.sections, idx, 1) })
                      }
                    >
                      <IconArrowDown size={16} stroke={1.5} />
                    </ActionIcon>
                  </Tooltip>
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
              </Group>
              <MultiSelect
                label="Bookmarks"
                placeholder="Pick bookmarks from your library"
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
              {section.bookmark_ids.length === 0 ? (
                <Text size="xs" c="dimmed">
                  No bookmarks yet — pick a few above for a dense glance row.
                </Text>
              ) : (
                <SimpleGrid cols={{ base: 1, xs: 2, sm: 3, lg: 4 }} spacing="xs">
                  {section.bookmark_ids.map((id) => {
                    const b = bookmarkById.get(id)
                    if (!b) {
                      return (
                        <Text key={id} size="xs" c="dimmed">
                          Unavailable bookmark
                        </Text>
                      )
                    }
                    return <GlanceTile key={id} bookmark={b} viewerUserId={viewerUserId} />
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
          description="Pin frequently used links into sections for a quick glance home."
          action={{
            label: 'Edit layout',
            onClick: startEdit,
            leftSection: <IconPencil {...iconSm} />,
          }}
        />
      ) : (
        <Stack gap="lg">
          {view.sections.map((section) => (
            <Stack key={section.id} gap="sm">
              <Title order={4}>{section.title}</Title>
              {section.items.length === 0 ? (
                <Text size="sm" c="dimmed">
                  No bookmarks in this section.
                </Text>
              ) : (
                <SimpleGrid cols={{ base: 1, xs: 2, sm: 3, md: 4, lg: 5, xl: 6 }} spacing="xs">
                  {section.items.map((item) => (
                    <GlanceTile
                      key={item.id}
                      bookmark={item.bookmark}
                      viewerUserId={viewerUserId}
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
