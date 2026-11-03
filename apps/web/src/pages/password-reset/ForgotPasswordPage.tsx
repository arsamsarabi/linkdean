import { useState, type ReactElement } from 'react'
import { Alert, Anchor, Button, Paper, Stack, TextInput } from '@mantine/core'
import { IconArrowLeft, IconMail } from '@tabler/icons-react'
import { useNavigate } from '@tanstack/react-router'
import { useForgotPassword } from '@/hooks'
import motion from '@/styles/motion.module.css'
import { AuthHeader } from './AuthHeader'

const iconSm = { size: 16, stroke: 1.5 } as const

export function ForgotPasswordPage(): ReactElement {
  const nav = useNavigate()
  const [email, setEmail] = useState('')
  const forgot = useForgotPassword()

  function submit(): void {
    if (forgot.isPending || !email.trim()) return
    forgot.mutate(email.trim())
  }

  const backToSignIn = (
    <Anchor component="button" type="button" size="sm" onClick={() => void nav({ to: '/login' })}>
      <IconArrowLeft size={14} stroke={1.5} style={{ verticalAlign: 'middle', marginRight: 4 }} />
      Back to sign in
    </Anchor>
  )

  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <AuthHeader title="Reset password" subtitle="We'll email you a link to set a new password." />
      <Paper p="xl" radius="md" withBorder>
        {forgot.data ? (
          <Stack gap="md">
            <Alert color="violet" variant="light" title="Check your email">
              If an account exists for that address, we've sent a link. It works for {forgot.data.minutes} minutes.
            </Alert>
            {backToSignIn}
          </Stack>
        ) : (
          <Stack gap="md">
            {forgot.isError ? (
              <Alert color="red" variant="light">
                {forgot.error.message}
              </Alert>
            ) : null}
            <TextInput
              label="Email"
              type="email"
              autoComplete="email"
              value={email}
              onChange={(e) => setEmail(e.currentTarget.value)}
              onKeyDown={(e) => e.key === 'Enter' && submit()}
            />
            <Button
              leftSection={<IconMail {...iconSm} />}
              loading={forgot.isPending}
              disabled={!email.trim()}
              onClick={submit}
            >
              Send reset link
            </Button>
            {backToSignIn}
          </Stack>
        )}
      </Paper>
    </Stack>
  )
}
