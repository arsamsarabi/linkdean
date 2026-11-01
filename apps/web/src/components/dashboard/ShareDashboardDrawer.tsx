import { useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Button,
  Drawer,
  Group,
  Select,
  Skeleton,
  Stack,
  Text,
  TextInput,
  Tooltip,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconTrash } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import {
  useCreateDashboardShare,
  useDashboardShares,
  useDeleteDashboardPendingShare,
  useDeleteDashboardShare,
  useUpdateDashboardPendingShare,
  useUpdateDashboardShare,
} from '@/hooks'
import type { ShareAccess } from '@/schemas'

type Props = {
  dashboardId: string | null
  dashboardTitle: string
  opened: boolean
  onClose: () => void
}

const accessOptions = [
  { value: 'read', label: 'Read-only' },
  { value: 'write', label: 'Read-write' },
]

export function ShareDashboardDrawer({
  dashboardId,
  dashboardTitle,
  opened,
  onClose,
}: Props): ReactElement {
  const [email, setEmail] = useState('')
  const [access, setAccess] = useState<ShareAccess>('read')

  const shares = useDashboardShares(opened ? dashboardId : null)
  const create = useCreateDashboardShare()
  const update = useUpdateDashboardShare()
  const remove = useDeleteDashboardShare()
  const updatePending = useUpdateDashboardPendingShare()
  const removePending = useDeleteDashboardPendingShare()

  async function onInvite(): Promise<void> {
    if (!dashboardId || !email.trim()) return
    try {
      const outcome = await create.mutateAsync({
        dashboardId,
        email: email.trim(),
        access,
      })
      setEmail('')
      notifications.show({
        message:
          outcome.status === 'pending'
            ? 'No account yet — access granted when they register'
            : 'Invite added',
      })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const activeShares = shares.data?.shares ?? []
  const pendingInvites = shares.data?.pending ?? []

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={`Share “${dashboardTitle}”`}
      position="right"
      size="md"
      padding="lg"
    >
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          Invite by email. Existing users get access now; unknown emails stay pending until they
          register. Read-write can rearrange sections and pins (not delete, share, or pin as
          default).
        </Text>

        <Stack gap="sm">
          <TextInput
            label="Email"
            placeholder="user@example.com"
            value={email}
            onChange={(e) => setEmail(e.currentTarget.value)}
          />
          <Select
            label="Access"
            data={accessOptions}
            value={access}
            onChange={(v) => setAccess((v as ShareAccess) || 'read')}
            allowDeselect={false}
          />
          <Button onClick={() => void onInvite()} disabled={!email.trim()} loading={create.isPending}>
            Invite
          </Button>
        </Stack>

        {shares.isError ? (
          <QueryErrorAlert error={shares.error} onRetry={() => void shares.refetch()} />
        ) : null}

        {shares.isLoading ? (
          <Stack gap="xs">
            <Skeleton height={28} />
            <Skeleton height={28} width="80%" />
          </Stack>
        ) : (
          <Stack gap="xs">
            {activeShares.length === 0 && pendingInvites.length === 0 ? (
              <Text size="sm" c="dimmed">
                No invitees yet.
              </Text>
            ) : null}
            {activeShares.map((s) => (
              <Group key={s.id} justify="space-between" wrap="nowrap">
                <Stack gap={0} style={{ minWidth: 0, flex: 1 }}>
                  <Text size="sm" truncate>
                    {s.invitee_display_name || s.invitee_email}
                  </Text>
                  {s.invitee_display_name ? (
                    <Text size="xs" c="dimmed" truncate>
                      {s.invitee_email}
                    </Text>
                  ) : null}
                </Stack>
                <Select
                  size="xs"
                  data={accessOptions}
                  value={s.access}
                  allowDeselect={false}
                  onChange={(v) => {
                    if (!dashboardId || !v) return
                    void update
                      .mutateAsync({
                        dashboardId,
                        userId: s.invitee_user_id,
                        access: v as ShareAccess,
                      })
                      .catch((e) => notifications.show({ color: 'red', message: String(e) }))
                  }}
                  w={130}
                />
                <Tooltip label="Revoke">
                  <ActionIcon
                    size="sm"
                    variant="subtle"
                    color="red"
                    aria-label="Revoke share"
                    loading={remove.isPending}
                    onClick={() => {
                      if (!dashboardId) return
                      void remove
                        .mutateAsync({ dashboardId, userId: s.invitee_user_id })
                        .then(() => notifications.show({ message: 'Access revoked' }))
                        .catch((e) => notifications.show({ color: 'red', message: String(e) }))
                    }}
                  >
                    <IconTrash size={14} stroke={1.5} />
                  </ActionIcon>
                </Tooltip>
              </Group>
            ))}
            {pendingInvites.map((p) => (
              <Group key={p.id} justify="space-between" wrap="nowrap">
                <Stack gap={0} style={{ minWidth: 0, flex: 1 }}>
                  <Text size="sm" truncate>
                    {p.invitee_email}
                  </Text>
                  <Text size="xs" c="dimmed">
                    Pending — claims on register
                  </Text>
                </Stack>
                <Select
                  size="xs"
                  data={accessOptions}
                  value={p.access}
                  allowDeselect={false}
                  onChange={(v) => {
                    if (!dashboardId || !v) return
                    void updatePending
                      .mutateAsync({
                        dashboardId,
                        pendingId: p.id,
                        access: v as ShareAccess,
                      })
                      .catch((e) => notifications.show({ color: 'red', message: String(e) }))
                  }}
                  w={130}
                />
                <Tooltip label="Cancel pending invite">
                  <ActionIcon
                    size="sm"
                    variant="subtle"
                    color="red"
                    aria-label="Cancel pending invite"
                    loading={removePending.isPending}
                    onClick={() => {
                      if (!dashboardId) return
                      void removePending
                        .mutateAsync({ dashboardId, pendingId: p.id })
                        .then(() => notifications.show({ message: 'Pending invite cancelled' }))
                        .catch((e) => notifications.show({ color: 'red', message: String(e) }))
                    }}
                  >
                    <IconTrash size={14} stroke={1.5} />
                  </ActionIcon>
                </Tooltip>
              </Group>
            ))}
          </Stack>
        )}
      </Stack>
    </Drawer>
  )
}
