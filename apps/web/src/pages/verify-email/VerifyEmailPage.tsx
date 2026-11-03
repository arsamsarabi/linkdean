import { useEffect, useState, type ReactElement } from 'react'
import { Alert, Button, Center, Loader, Paper, Stack, Text } from '@mantine/core'
import { IconArrowRight, IconLogin, IconMail, IconUser } from '@tabler/icons-react'
import { useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { api, ApiError } from '@/api'
import { useResendVerification, useSessionCheck } from '@/hooks'
import { oncePerKey } from '@/services'
import motion from '@/styles/motion.module.css'
import { AuthHeader } from '../password-reset/AuthHeader'
import { VerifyEmailGate } from './VerifyEmailGate'

const iconSm = { size: 16, stroke: 1.5 } as const

type Link = { token: string; change: boolean }

/** The token rides in the fragment (`#token=…`), so it never reaches server logs or a Referer header. */
function linkFromHash(): Link {
  const params = new URLSearchParams(window.location.hash.slice(1))
  return { token: params.get('token')?.trim() ?? '', change: params.get('change') === '1' }
}

function isSpentLink(e: unknown): boolean {
  return e instanceof ApiError && e.status === 400 && e.message.includes('expired or was already used')
}

/**
 * `/verify-email#token=…` verifies an address and `#token=…&change=1` confirms an email change.
 * Without a token this is the hard gate's screen for an unverified account.
 */
export function VerifyEmailPage(): ReactElement {
  const [link, setLink] = useState(linkFromHash)

  // Drop the token from the address bar once it has been read. A link pasted into this tab
  // (say, on the gate screen) changes only the fragment, so the page does not reload: read it.
  useEffect(() => {
    function clearHash(): void {
      if (!window.location.hash) return
      window.history.replaceState(window.history.state, '', window.location.pathname + window.location.search)
    }
    function onHashChange(): void {
      const next = linkFromHash()
      clearHash()
      if (next.token) setLink(next)
    }
    clearHash()
    window.addEventListener('hashchange', onHashChange)
    return () => window.removeEventListener('hashchange', onHashChange)
  }, [])

  if (!link.token) return <VerifyEmailGate />
  return (
    <VerifyEmailLink
      key={`${link.change ? 'change' : 'verify'}:${link.token}`}
      token={link.token}
      change={link.change}
    />
  )
}

type Outcome = { state: 'pending' } | { state: 'done' } | { state: 'failed'; error: unknown }

function VerifyEmailLink({ token, change }: Link): ReactElement {
  const nav = useNavigate()
  const qc = useQueryClient()
  const [outcome, setOutcome] = useState<Outcome>({ state: 'pending' })
  // Ask about the session only after the link is spent, so a stale session cannot interrupt it.
  const session = useSessionCheck(outcome.state !== 'pending')
  const signedIn = session.isSuccess
  const resend = useResendVerification()

  useEffect(() => {
    let live = true
    const key = `${change ? 'change' : 'verify'}:${token}`
    oncePerKey(key, () => (change ? api.confirmEmailChange(token) : api.verifyEmail(token))).then(
      () => {
        if (!live) return
        setOutcome({ state: 'done' })
        void qc.invalidateQueries({ queryKey: ['me'] })
      },
      (error: unknown) => {
        if (live) setOutcome({ state: 'failed', error })
      }
    )
    return () => {
      live = false
    }
  }, [change, qc, token])

  if (outcome.state === 'pending') {
    return (
      <Center mih="70vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  const continueButton = signedIn ? (
    <Button rightSection={<IconArrowRight {...iconSm} />} onClick={() => void nav({ to: '/library' })}>
      Continue to your library
    </Button>
  ) : (
    <Button leftSection={<IconLogin {...iconSm} />} onClick={() => void nav({ to: '/login' })}>
      Sign in
    </Button>
  )

  if (outcome.state === 'done') {
    return (
      <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
        <AuthHeader
          title={change ? 'Email changed' : 'Email verified'}
          subtitle={change ? 'Sign in with your new address from now on.' : 'People can now share with you.'}
        />
        <Paper p="xl" radius="md" withBorder>
          {continueButton}
        </Paper>
      </Stack>
    )
  }

  const message = isSpentLink(outcome.error)
    ? 'This link has expired or was already used.'
    : outcome.error instanceof Error
      ? outcome.error.message
      : 'Something went wrong. Try again.'

  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <AuthHeader
        title={change ? 'Change email' : 'Verify email'}
        subtitle={change ? 'Confirm your new address.' : 'Confirm your address.'}
      />
      <Paper p="xl" radius="md" withBorder>
        <Stack gap="md">
          <Alert color="red" variant="light" title="Link not valid">
            {message}
          </Alert>
          {resend.data?.verified ? (
            <Text size="sm">Your email is already verified.</Text>
          ) : resend.isSuccess ? (
            <Text size="sm">We sent a new link. It works for {resend.data.hours ?? 24} hours.</Text>
          ) : null}
          {resend.isError ? (
            <Alert color="red" variant="light">
              {resend.error.message}
            </Alert>
          ) : null}
          {!signedIn ? (
            continueButton
          ) : change ? (
            <Button leftSection={<IconUser {...iconSm} />} onClick={() => void nav({ to: '/profile' })}>
              Back to your profile
            </Button>
          ) : resend.isSuccess ? (
            continueButton
          ) : (
            <Button leftSection={<IconMail {...iconSm} />} loading={resend.isPending} onClick={() => resend.mutate()}>
              Send a new link
            </Button>
          )}
        </Stack>
      </Paper>
    </Stack>
  )
}
