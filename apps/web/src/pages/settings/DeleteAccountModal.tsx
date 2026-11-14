import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, List, Modal, PasswordInput, Stack, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconLogin2, IconTrash } from '@tabler/icons-react'
import { ApiError } from '@/api'
import { useScheduleAccountDeletion } from '@/hooks'
import { OIDC_REAUTH_URL, REAUTH_REQUIRED_MESSAGE } from '@/lib'

type Props = {
  email: string
  hasPassword: boolean
  /** The attached single sign-on's host, named in the sign-in-again step. */
  ssoProvider?: string | null
  /** This session just signed in again with single sign-on, so a passwordless account may delete. */
  reauthed?: boolean
  /** Why the last sign-in-again failed. */
  signInError?: string
  onClose: () => void
}

function errorMessage(e: Error): string {
  if (e instanceof ApiError && e.status === 401) return 'Wrong password.'
  return e.message
}

export function formatDeletionDate(iso: string): string {
  return new Date(iso).toLocaleDateString(undefined, { dateStyle: 'long' })
}

export function DeleteAccountModal({
  email,
  hasPassword,
  ssoProvider,
  reauthed = false,
  signInError,
  onClose,
}: Props): ReactElement {
  const schedule = useScheduleAccountDeletion()
  const [confirmEmail, setConfirmEmail] = useState('')
  const [password, setPassword] = useState('')

  // Without a password, a fresh single sign-on proves it is the owner. It lasts a few minutes,
  // so a late submit can still be refused and asks again.
  const reauthExpired = schedule.error?.message === REAUTH_REQUIRED_MESSAGE
  const needsReauth = !hasPassword && (!reauthed || reauthExpired)
  const emailOk = confirmEmail.trim().toLowerCase() === email.toLowerCase()
  const ready = emailOk && (!hasPassword || !!password) && !needsReauth

  function submit(): void {
    if (schedule.isPending || !ready) return
    schedule.mutate(
      { confirm_email: confirmEmail.trim(), ...(hasPassword ? { password } : {}) },
      {
        onSuccess: (res) => {
          notifications.show({ message: `Account will be deleted on ${formatDeletionDate(res.deletion_scheduled_at)}` })
          onClose()
        },
      }
    )
  }

  return (
    <Modal opened onClose={onClose} title="Delete account">
      <Stack>
        <Text size="sm">
          Your account is deleted after a 7-day grace period. Until then you can sign in and cancel from Config options.
        </Text>
        <List size="sm" c="dimmed" spacing={4}>
          <List.Item>Bookmarks, categories, vaults, files, API keys and sessions are erased.</List.Item>
          <List.Item>Shared folders you own are deleted. Everyone you shared them with loses access.</List.Item>
          <List.Item>Export your data first (Import / Export) if you want a copy.</List.Item>
        </List>
        {schedule.isError && !reauthExpired ? (
          <Alert color="red" variant="light">
            {errorMessage(schedule.error)}
          </Alert>
        ) : null}
        {needsReauth && signInError ? (
          <Alert color="red" variant="light">
            {signInError}
          </Alert>
        ) : null}
        {needsReauth ? (
          <Alert color="violet" variant="light" title="Sign in again first">
            <Text size="sm">
              {reauthExpired ? 'That sign-in is more than a few minutes old. ' : ''}
              To confirm it is you, sign in again with {ssoProvider ?? 'single sign-on'}. You come back here to finish.
            </Text>
          </Alert>
        ) : null}
        {/* Typed text would be lost on the way to the provider, so ask for it after. */}
        {needsReauth ? null : (
          <TextInput
            label={`Type ${email} to confirm`}
            autoComplete="off"
            data-autofocus
            value={confirmEmail}
            onChange={(e) => setConfirmEmail(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && submit()}
          />
        )}
        {hasPassword ? (
          <PasswordInput
            label="Current password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.currentTarget.value)}
            onKeyDown={(e) => e.key === 'Enter' && submit()}
          />
        ) : null}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Keep account
          </Button>
          {needsReauth ? (
            <Button component="a" href={OIDC_REAUTH_URL} leftSection={<IconLogin2 size={16} stroke={1.5} />}>
              Sign in again
            </Button>
          ) : (
            <Button
              color="red"
              leftSection={<IconTrash size={16} stroke={1.5} />}
              loading={schedule.isPending}
              disabled={!ready}
              onClick={submit}
            >
              Delete account
            </Button>
          )}
        </Group>
      </Stack>
    </Modal>
  )
}
