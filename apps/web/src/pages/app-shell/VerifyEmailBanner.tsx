import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Text } from '@mantine/core'
import { IconCheck, IconMail } from '@tabler/icons-react'
import { useResendVerification } from '@/hooks'

const dismissedKey = 'linkdean_verify_banner_dismissed'

function wasDismissed(): boolean {
  try {
    return sessionStorage.getItem(dismissedKey) === '1'
  } catch {
    return false
  }
}

type Props = {
  /** The account's address is not verified yet. */
  unverified: boolean
  /** Outbound mail is on; without it nobody can verify, so the banner stays hidden. */
  mailEnabled: boolean
}

/** Asks an unverified account to verify. Dismissing it lasts for this browser session. */
export function VerifyEmailBanner({ unverified, mailEnabled }: Props): ReactElement | null {
  const [dismissed, setDismissed] = useState(wasDismissed)
  const resend = useResendVerification()

  if (!unverified || !mailEnabled || dismissed) return null

  function dismiss(): void {
    setDismissed(true)
    try {
      sessionStorage.setItem(dismissedKey, '1')
    } catch {
      // Storage blocked: it stays dismissed until the next page load.
    }
  }

  return (
    <Alert color="violet" variant="light" py="xs" mb="md" withCloseButton closeButtonLabel="Dismiss" onClose={dismiss}>
      <Group justify="space-between" gap="xs" wrap="wrap">
        <Text size="sm">Verify your email so people can share with you.</Text>
        <Group gap="xs">
          {resend.isError ? (
            <Text size="sm" c="red">
              {resend.error.message}
            </Text>
          ) : null}
          <Button
            size="xs"
            variant="light"
            leftSection={resend.isSuccess ? <IconCheck size={14} /> : <IconMail size={14} />}
            loading={resend.isPending}
            disabled={resend.isSuccess}
            onClick={() => resend.mutate()}
          >
            {resend.isSuccess ? 'Sent' : 'Resend email'}
          </Button>
        </Group>
      </Group>
    </Alert>
  )
}
