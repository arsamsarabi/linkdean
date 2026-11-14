import { useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Alert,
  Button,
  Group,
  Modal,
  Paper,
  PasswordInput,
  Stack,
  Text,
  TextInput,
  ThemeIcon,
  Title,
  Tooltip,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconFingerprint, IconLogin2, IconPencil, IconPlus, IconTrash } from '@tabler/icons-react'
import { ApiError } from '@/api'
import { ConfirmModal } from '@/components'
import { useAddPasskey, useDeletePasskey, usePasskeys, useRenamePasskey } from '@/hooks'
import {
  isPasskeyCancel,
  OIDC_REAUTH_PROFILE_URL,
  passkeysSupported,
  REAUTH_REQUIRED_MESSAGE,
  suggestedPasskeyName,
} from '@/lib'
import type { Passkey } from '@/schemas'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  hasPassword: boolean
  /** Back from signing in again with single sign-on: open the add dialog to finish. */
  reauthed?: boolean
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, { dateStyle: 'medium' })
}

/** Passkeys that sign in to this account without a password or email. */
export function ProfilePasskeysCard({ hasPassword, reauthed = false }: Props): ReactElement {
  const list = usePasskeys()
  const add = useAddPasskey()
  const rename = useRenamePasskey()
  const remove = useDeletePasskey()
  const [supported] = useState(passkeysSupported)
  const [adding, setAdding] = useState(reauthed)
  const [name, setName] = useState(suggestedPasskeyName)
  const [password, setPassword] = useState('')
  const [editing, setEditing] = useState<Passkey | null>(null)
  const [editName, setEditName] = useState('')
  const [removing, setRemoving] = useState<Passkey | null>(null)

  const reauthExpired = add.error?.message === REAUTH_REQUIRED_MESSAGE
  const needsReauth = !hasPassword && (!reauthed || reauthExpired)
  const addError = add.error && !isPasskeyCancel(add.error) && !reauthExpired ? add.error : null

  function closeAdd(): void {
    setAdding(false)
    setPassword('')
    add.reset()
  }

  function submitAdd(): void {
    if (add.isPending || !name.trim() || (hasPassword && !password)) return
    add.mutate(
      { name: name.trim(), ...(hasPassword ? { password } : {}) },
      {
        onSuccess: () => {
          notifications.show({ message: 'Passkey added' })
          closeAdd()
        },
      }
    )
  }

  const passkeys = list.data?.passkeys ?? []

  return (
    <Paper p="xl" withBorder>
      <Stack gap="md">
        <Group gap="sm" justify="space-between" wrap="nowrap">
          <Group gap="sm" wrap="nowrap">
            <ThemeIcon size={28} radius="md" variant="light" color="violet">
              <IconFingerprint size={16} stroke={1.5} />
            </ThemeIcon>
            <Title order={4}>Passkeys</Title>
          </Group>
          {supported ? (
            <Button variant="light" size="xs" leftSection={<IconPlus {...iconSm} />} onClick={() => setAdding(true)}>
              Add passkey
            </Button>
          ) : null}
        </Group>
        <Text size="sm" c="dimmed">
          Sign in with your fingerprint, face or device PIN instead of a password.
          {supported ? '' : ' This browser does not support passkeys.'}
        </Text>
        {passkeys.length === 0 && list.isSuccess ? (
          <Text size="sm" c="dimmed">
            No passkeys yet.
          </Text>
        ) : null}
        {passkeys.map((p) => (
          <Group key={p.id} justify="space-between" wrap="nowrap" gap="sm">
            <Stack gap={0} style={{ minWidth: 0 }}>
              <Text size="sm" fw={500} truncate>
                {p.name}
              </Text>
              <Text size="xs" c="dimmed">
                Added {formatDate(p.created_at)} ·{' '}
                {p.last_used_at ? `last used ${formatDate(p.last_used_at)}` : 'never used'}
              </Text>
            </Stack>
            <Group gap={4} wrap="nowrap">
              <Tooltip label="Rename">
                <ActionIcon
                  variant="subtle"
                  color="gray"
                  aria-label={`Rename ${p.name}`}
                  onClick={() => {
                    setEditing(p)
                    setEditName(p.name)
                  }}
                >
                  <IconPencil {...iconSm} />
                </ActionIcon>
              </Tooltip>
              <Tooltip label="Remove">
                <ActionIcon variant="subtle" color="red" aria-label={`Remove ${p.name}`} onClick={() => setRemoving(p)}>
                  <IconTrash {...iconSm} />
                </ActionIcon>
              </Tooltip>
            </Group>
          </Group>
        ))}
      </Stack>

      <Modal opened={adding} onClose={closeAdd} title="Add a passkey">
        <Stack>
          {needsReauth ? (
            <Alert color="violet" variant="light" title="Sign in again first">
              <Text size="sm">
                {reauthExpired ? 'That sign-in is more than a few minutes old. ' : ''}
                To confirm it is you, sign in again with single sign-on. You come back here to finish.
              </Text>
            </Alert>
          ) : (
            <>
              <TextInput
                label="Name"
                description="So you can tell your passkeys apart"
                maxLength={64}
                value={name}
                data-autofocus
                onChange={(e) => setName(e.currentTarget.value)}
                onKeyDown={(e) => e.key === 'Enter' && submitAdd()}
              />
              {hasPassword ? (
                <PasswordInput
                  label="Current password"
                  autoComplete="current-password"
                  value={password}
                  onChange={(e) => setPassword(e.currentTarget.value)}
                  onKeyDown={(e) => e.key === 'Enter' && submitAdd()}
                />
              ) : null}
            </>
          )}
          {addError ? (
            <Alert color="red" variant="light">
              {addError instanceof ApiError && addError.status === 401 ? 'Wrong password.' : addError.message}
            </Alert>
          ) : null}
          <Group justify="flex-end">
            <Button variant="default" onClick={closeAdd}>
              Cancel
            </Button>
            {needsReauth ? (
              <Button component="a" href={OIDC_REAUTH_PROFILE_URL} leftSection={<IconLogin2 {...iconSm} />}>
                Sign in again
              </Button>
            ) : (
              <Button
                leftSection={<IconFingerprint {...iconSm} />}
                loading={add.isPending}
                disabled={!name.trim() || (hasPassword && !password)}
                onClick={submitAdd}
              >
                Add passkey
              </Button>
            )}
          </Group>
        </Stack>
      </Modal>

      <Modal opened={!!editing} onClose={() => setEditing(null)} title="Rename passkey">
        <Stack>
          <TextInput
            label="Name"
            maxLength={64}
            value={editName}
            data-autofocus
            onChange={(e) => setEditName(e.currentTarget.value)}
          />
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setEditing(null)}>
              Cancel
            </Button>
            <Button
              loading={rename.isPending}
              disabled={!editName.trim()}
              onClick={() =>
                editing &&
                rename.mutate(
                  { id: editing.id, name: editName.trim() },
                  {
                    onSuccess: () => setEditing(null),
                    onError: (e) => notifications.show({ color: 'red', message: e.message }),
                  }
                )
              }
            >
              Save
            </Button>
          </Group>
        </Stack>
      </Modal>

      <ConfirmModal
        opened={!!removing}
        onClose={() => setRemoving(null)}
        title="Remove passkey"
        body={`"${removing?.name ?? ''}" will no longer sign in to this account. Remove it from the device too.`}
        confirmLabel="Remove"
        loading={remove.isPending}
        onConfirm={() =>
          removing &&
          remove.mutate(removing.id, {
            onSuccess: () => {
              setRemoving(null)
              notifications.show({ message: 'Passkey removed' })
            },
            onError: (e) => notifications.show({ color: 'red', message: e.message }),
          })
        }
      />
    </Paper>
  )
}
