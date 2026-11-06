import { type ReactElement } from 'react'
import { ActionIcon, Box, Button, Drawer, Group, LoadingOverlay, Stack, Text, Title, Tooltip } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconExternalLink, IconLinkOff, IconRefresh, IconTrash } from '@tabler/icons-react'
import { useQueryClient } from '@tanstack/react-query'
import { QueryErrorAlert } from '@/components/query-error-alert'
import {
  brokenLinksQueryKey,
  useBrokenLinks,
  useDeleteBookmark,
  usePrefetchBrokenLinks,
  useRecheckBookmarkLink,
} from '@/hooks'
import { useDrawerSize } from '@/hooks/render'
import type { Bookmark } from '@/schemas'

type Props = {
  opened: boolean
  onClose: () => void
  onEdit?: (id: string) => void
}

function statusLabel(b: Bookmark): string {
  const code = b.link_check_status_code
  const status = b.link_check_status ?? 'broken'
  if (code != null) return `${status} · HTTP ${code}`
  if (b.link_check_error) return `${status} · ${b.link_check_error}`
  return status
}

export function BrokenLinksDrawer({ opened, onClose, onEdit }: Props): ReactElement {
  const drawerSize = useDrawerSize()
  const broken = useBrokenLinks(opened)
  const recheck = useRecheckBookmarkLink()
  const remove = useDeleteBookmark()
  const qc = useQueryClient()

  async function onRemove(b: Bookmark): Promise<void> {
    try {
      await remove.mutateAsync(b.id)
      await qc.invalidateQueries({ queryKey: brokenLinksQueryKey })
      notifications.show({ message: `Moved “${b.title || b.url}” to Deleted items` })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onRecheck(id: string): Promise<void> {
    try {
      const updated = await recheck.mutateAsync(id)
      if (updated.link_check_status === 'ok' || updated.link_check_status === 'unverifiable') {
        notifications.show({ message: 'Link looks reachable again' })
      } else {
        notifications.show({
          color: 'yellow',
          message: `Still ${updated.link_check_status ?? 'broken'}`,
        })
      }
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const items = broken.data ?? []

  return (
    <Drawer opened={opened} onClose={onClose} title="Broken links" position="right" size={drawerSize} padding="lg">
      <Box pos="relative">
        <LoadingOverlay visible={broken.isLoading} zIndex={100} overlayProps={{ blur: 1 }} />
        <Stack gap="md">
          {broken.isError ? (
            <QueryErrorAlert
              error={broken.error}
              onRetry={() => void broken.refetch()}
              title="Broken-link list failed"
            />
          ) : null}
          {!broken.isLoading && !broken.isError && items.length === 0 ? (
            <Text size="sm" c="dimmed">
              No confirmed broken or unreachable bookmarks. Vault links are skipped; soft failures need two failed
              checks (or a 404/410) before they appear here.
            </Text>
          ) : null}
          {items.length > 0 ? (
            <>
              <Title order={5}>
                {items.length} problem{items.length === 1 ? '' : 's'}
              </Title>
              {items.map((b) => (
                <Stack
                  key={b.id}
                  gap="xs"
                  p="md"
                  style={{
                    border: '1px solid var(--mantine-color-default-border)',
                    borderRadius: 8,
                  }}
                >
                  <Text size="sm" fw={600} lineClamp={2}>
                    {b.title || b.url}
                  </Text>
                  <Text size="xs" c="dimmed" lineClamp={2}>
                    {b.url}
                  </Text>
                  <Text size="xs" c="red">
                    {statusLabel(b)}
                    {b.link_checked_at ? ` · checked ${new Date(b.link_checked_at).toLocaleString()}` : ''}
                  </Text>
                  <Group gap="xs" justify="flex-end">
                    <Button
                      size="compact-sm"
                      variant="subtle"
                      color="red"
                      mr="auto"
                      loading={remove.isPending && remove.variables === b.id}
                      leftSection={<IconTrash size={14} stroke={1.5} />}
                      aria-label={`Remove bookmark ${b.title || b.url}`}
                      onClick={() => void onRemove(b)}
                    >
                      Remove bookmark
                    </Button>
                    <Button
                      size="compact-sm"
                      variant="subtle"
                      component="a"
                      href={b.url}
                      target="_blank"
                      rel="noreferrer"
                      leftSection={<IconExternalLink size={14} stroke={1.5} />}
                    >
                      Open
                    </Button>
                    {onEdit ? (
                      <Button size="compact-sm" variant="default" onClick={() => onEdit(b.id)}>
                        Edit
                      </Button>
                    ) : null}
                    <Button
                      size="compact-sm"
                      variant="light"
                      loading={recheck.isPending && recheck.variables === b.id}
                      leftSection={<IconRefresh size={14} stroke={1.5} />}
                      onClick={() => void onRecheck(b.id)}
                    >
                      Recheck
                    </Button>
                  </Group>
                </Stack>
              ))}
            </>
          ) : null}
        </Stack>
      </Box>
    </Drawer>
  )
}

export function BrokenLinksButton({ onClick }: { onClick: () => void }): ReactElement {
  const prefetch = usePrefetchBrokenLinks()
  return (
    <Tooltip label="Broken links" position="bottom">
      <ActionIcon
        variant="subtle"
        size="lg"
        radius="md"
        aria-label="Broken links"
        onClick={onClick}
        onMouseEnter={prefetch}
        onFocus={prefetch}
        onPointerDown={prefetch}
      >
        <IconLinkOff size={20} stroke={1.5} />
      </ActionIcon>
    </Tooltip>
  )
}
