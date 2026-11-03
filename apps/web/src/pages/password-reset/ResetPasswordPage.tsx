import { useEffect, useState, type ReactElement } from 'react'
import { Alert, Button, Checkbox, Paper, PasswordInput, Stack } from '@mantine/core'
import { IconKey, IconMail } from '@tabler/icons-react'
import { useNavigate } from '@tanstack/react-router'
import { ApiError } from '@/api'
import { useResetPassword } from '@/hooks'
import { MIN_SECRET_LENGTH, validatePasswordPair } from '@/services'
import motion from '@/styles/motion.module.css'
import { AuthHeader } from './AuthHeader'

const iconSm = { size: 16, stroke: 1.5 } as const

const MESSAGES = {
  tooShort: `Use at least ${MIN_SECRET_LENGTH} characters.`,
  mismatch: 'The passwords do not match.',
}

/** The token rides in the fragment (`#token=…`), so it never reaches server logs or a Referer header. */
function tokenFromHash(): string {
  return new URLSearchParams(window.location.hash.slice(1)).get('token')?.trim() ?? ''
}

function isSpentLink(e: unknown): boolean {
  return e instanceof ApiError && e.status === 400 && e.message.includes('expired or was already used')
}

export function ResetPasswordPage(): ReactElement {
  const nav = useNavigate()
  const [token] = useState(tokenFromHash)
  const [password, setPassword] = useState('')
  const [password2, setPassword2] = useState('')
  const [revokeApiKeys, setRevokeApiKeys] = useState(false)
  const [formError, setFormError] = useState<string | null>(null)
  const reset = useResetPassword()

  // Drop the token from the address bar once it has been read.
  useEffect(() => {
    if (!window.location.hash) return
    window.history.replaceState(window.history.state, '', window.location.pathname + window.location.search)
  }, [])

  async function submit(): Promise<void> {
    if (reset.isPending) return
    const check = validatePasswordPair(password, password2, MESSAGES)
    if (!check.ok) {
      setFormError(check.message)
      return
    }
    setFormError(null)
    try {
      await reset.mutateAsync({ token, password, revoke_api_keys: revokeApiKeys })
      void nav({ to: '/library' })
    } catch {
      // Shown from the mutation's error below.
    }
  }

  const spent = !token || isSpentLink(reset.error)

  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <AuthHeader title="New password" subtitle="Choose a new password for your account." />
      <Paper p="xl" radius="md" withBorder>
        {spent ? (
          <Stack gap="md">
            <Alert color="red" variant="light" title="Link not valid">
              This link has expired or was already used.
            </Alert>
            <Button leftSection={<IconMail {...iconSm} />} onClick={() => void nav({ to: '/forgot-password' })}>
              Request a new link
            </Button>
          </Stack>
        ) : (
          <Stack gap="md">
            {formError || reset.isError ? (
              <Alert color="red" variant="light">
                {formError ?? reset.error?.message}
              </Alert>
            ) : null}
            <PasswordInput
              label="New password"
              description={`At least ${MIN_SECRET_LENGTH} characters.`}
              autoComplete="new-password"
              value={password}
              onChange={(e) => setPassword(e.currentTarget.value)}
            />
            <PasswordInput
              label="Repeat new password"
              autoComplete="new-password"
              value={password2}
              onChange={(e) => setPassword2(e.currentTarget.value)}
              onKeyDown={(e) => e.key === 'Enter' && void submit()}
            />
            <Checkbox
              label="Also revoke my API keys"
              checked={revokeApiKeys}
              onChange={(e) => setRevokeApiKeys(e.currentTarget.checked)}
            />
            <Button leftSection={<IconKey {...iconSm} />} loading={reset.isPending} onClick={() => void submit()}>
              Set new password
            </Button>
          </Stack>
        )}
      </Paper>
    </Stack>
  )
}
