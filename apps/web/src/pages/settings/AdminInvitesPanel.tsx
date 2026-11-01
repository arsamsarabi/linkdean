import { useState, type ReactElement } from 'react'
import { Button, Group, NumberInput, Paper, Stack, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCopy, IconTicket } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components'
import { useCreateRegistrationInvite, useRegistrationInvites, useRevokeRegistrationInvite } from '@/hooks'
import { SettingsSkeleton } from './SettingsSkeleton'

export function AdminInvitesPanel(): ReactElement {
  const invites = useRegistrationInvites(true)
  const createInvite = useCreateRegistrationInvite()
  const revokeInvite = useRevokeRegistrationInvite()
  const [inviteEmail, setInviteEmail] = useState('')
  const [inviteDays, setInviteDays] = useState(7)
  const [freshInviteCode, setFreshInviteCode] = useState<string | null>(null)

  return (
    <Paper p="xl" withBorder>
      <Stack gap="md">
        <div>
          <Text fw={500}>Registration invites</Text>
          <Text size="sm" c="dimmed">
            Issue one-time codes when registration is closed. Share the link; email delivery comes later with SMTP.
          </Text>
        </div>
        <Group align="flex-end" wrap="wrap" gap="sm">
          <TextInput
            label="Email (optional)"
            placeholder="bind invite to address"
            value={inviteEmail}
            onChange={(e) => setInviteEmail(e.currentTarget.value)}
            style={{ flex: 1, minWidth: 180 }}
          />
          <NumberInput
            label="Expires (days)"
            min={1}
            max={90}
            value={inviteDays}
            onChange={(v) => setInviteDays(typeof v === 'number' ? v : 7)}
            w={120}
          />
          <Button
            leftSection={<IconTicket size={16} stroke={1.5} />}
            loading={createInvite.isPending}
            onClick={() => {
              createInvite.mutate(
                {
                  email: inviteEmail.trim() || undefined,
                  expires_in_days: inviteDays,
                },
                {
                  onSuccess: (res) => {
                    setFreshInviteCode(res.code)
                    setInviteEmail('')
                    notifications.show({ message: 'Invite created — copy the code now' })
                  },
                  onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                }
              )
            }}
          >
            Create invite
          </Button>
        </Group>
        {freshInviteCode ? (
          <Paper p="md" withBorder radius="md">
            <Stack gap="xs">
              <Text size="sm" fw={500}>
                New invite (shown once)
              </Text>
              <Text size="xs" ff="monospace" style={{ wordBreak: 'break-all' }}>
                {freshInviteCode}
              </Text>
              <Group gap="xs">
                <Button
                  size="compact-sm"
                  variant="light"
                  leftSection={<IconCopy size={14} stroke={1.5} />}
                  onClick={() => {
                    void navigator.clipboard.writeText(freshInviteCode)
                    notifications.show({ message: 'Code copied' })
                  }}
                >
                  Copy code
                </Button>
                <Button
                  size="compact-sm"
                  variant="light"
                  leftSection={<IconCopy size={14} stroke={1.5} />}
                  onClick={() => {
                    const url = `${window.location.origin}/register?invite=${encodeURIComponent(freshInviteCode)}`
                    void navigator.clipboard.writeText(url)
                    notifications.show({ message: 'Invite link copied' })
                  }}
                >
                  Copy link
                </Button>
              </Group>
            </Stack>
          </Paper>
        ) : null}
        {invites.isError ? <QueryErrorAlert error={invites.error} onRetry={() => void invites.refetch()} /> : null}
        {invites.isLoading ? <SettingsSkeleton /> : null}
        {(invites.data?.invites ?? []).length === 0 && !invites.isLoading ? (
          <Text size="sm" c="dimmed">
            No invites yet.
          </Text>
        ) : null}
        {(invites.data?.invites ?? []).map((inv) => (
          <Group key={inv.id} justify="space-between" align="flex-start" wrap="wrap" gap="sm">
            <Stack gap={2}>
              <Text size="sm" fw={500} ff="monospace">
                {inv.code_prefix}…
              </Text>
              <Text size="xs" c="dimmed">
                {inv.status}
                {inv.email ? ` · ${inv.email}` : ''}
                {' · '}expires {new Date(inv.expires_at).toLocaleDateString()}
              </Text>
            </Stack>
            {inv.status === 'pending' ? (
              <Button
                size="compact-sm"
                variant="subtle"
                color="red"
                loading={revokeInvite.isPending}
                onClick={() => {
                  revokeInvite.mutate(inv.id, {
                    onSuccess: () => notifications.show({ message: 'Invite revoked' }),
                    onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                  })
                }}
              >
                Revoke
              </Button>
            ) : null}
          </Group>
        ))}
      </Stack>
    </Paper>
  )
}
