import { type ReactElement } from 'react'
import { ActionIcon, Button, Group, Select, Skeleton, Stack, Text, TextInput, Tooltip } from '@mantine/core'
import { IconTrash } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import type { ShareAccess } from '@/schemas'

export const SHARE_ACCESS_OPTIONS = [
  { value: 'read', label: 'Read-only' },
  { value: 'write', label: 'Read-write' },
] as const

export type ShareInviteActiveRow = {
  id: string
  invitee_user_id: string
  invitee_email: string
  invitee_display_name?: string | null
  access: ShareAccess
}

export type ShareInvitePendingRow = {
  id: string
  invitee_email: string
  access: ShareAccess
}

type Props = {
  description: string
  email: string
  access: ShareAccess
  onEmailChange: (email: string) => void
  onAccessChange: (access: ShareAccess) => void
  onInvite: () => void
  invitePending: boolean
  sharesError: unknown
  onRetryShares: () => void
  sharesLoading: boolean
  activeShares: ShareInviteActiveRow[]
  pendingInvites: ShareInvitePendingRow[]
  updateBusy?: boolean
  removeBusy?: boolean
  onUpdateAccess: (userId: string, access: ShareAccess) => void
  onRevoke: (userId: string) => void
  onUpdatePendingAccess: (pendingId: string, access: ShareAccess) => void
  onCancelPending: (pendingId: string) => void
  /** Extra content after the invitee list (e.g. public-link controls). */
  children?: React.ReactNode
}

export function ShareInvitePanel({
  description,
  email,
  access,
  onEmailChange,
  onAccessChange,
  onInvite,
  invitePending,
  sharesError,
  onRetryShares,
  sharesLoading,
  activeShares,
  pendingInvites,
  updateBusy,
  removeBusy,
  onUpdateAccess,
  onRevoke,
  onUpdatePendingAccess,
  onCancelPending,
  children,
}: Props): ReactElement {
  return (
    <Stack gap="md">
      <Text size="sm" c="dimmed">
        {description}
      </Text>

      <Stack gap="sm">
        <TextInput
          label="Email"
          placeholder="user@example.com"
          value={email}
          onChange={(e) => onEmailChange(e.currentTarget.value)}
        />
        <Select
          label="Access"
          data={[...SHARE_ACCESS_OPTIONS]}
          value={access}
          onChange={(v) => onAccessChange((v as ShareAccess) || 'read')}
          allowDeselect={false}
        />
        <Button onClick={onInvite} disabled={!email.trim()} loading={invitePending}>
          Invite
        </Button>
      </Stack>

      {sharesError ? <QueryErrorAlert error={sharesError} onRetry={onRetryShares} /> : null}

      {sharesLoading ? (
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
                data={[...SHARE_ACCESS_OPTIONS]}
                value={s.access}
                allowDeselect={false}
                onChange={(v) => {
                  if (!v) return
                  onUpdateAccess(s.invitee_user_id, v as ShareAccess)
                }}
                w={130}
              />
              <Tooltip label="Revoke">
                <ActionIcon
                  size="sm"
                  variant="subtle"
                  color="red"
                  aria-label="Revoke share"
                  loading={removeBusy}
                  onClick={() => onRevoke(s.invitee_user_id)}
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
                data={[...SHARE_ACCESS_OPTIONS]}
                value={p.access}
                allowDeselect={false}
                onChange={(v) => {
                  if (!v) return
                  onUpdatePendingAccess(p.id, v as ShareAccess)
                }}
                w={130}
              />
              <Tooltip label="Cancel pending invite">
                <ActionIcon
                  size="sm"
                  variant="subtle"
                  color="red"
                  aria-label="Cancel pending invite"
                  loading={updateBusy || removeBusy}
                  onClick={() => onCancelPending(p.id)}
                >
                  <IconTrash size={14} stroke={1.5} />
                </ActionIcon>
              </Tooltip>
            </Group>
          ))}
        </Stack>
      )}

      {children}
    </Stack>
  )
}
