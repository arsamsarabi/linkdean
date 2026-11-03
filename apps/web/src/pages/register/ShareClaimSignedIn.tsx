import { useEffect, useState, type ReactElement } from 'react'
import { Alert, Button, Center, Loader, Paper, Stack } from '@mantine/core'
import { IconArrowRight } from '@tabler/icons-react'
import { useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { api } from '@/api'
import { oncePerKey } from '@/services'
import motion from '@/styles/motion.module.css'
import { AuthHeader } from '../password-reset/AuthHeader'

type Outcome = { state: 'pending' } | { state: 'done' } | { state: 'failed'; message: string }

/** An invite link opened while signed in: claim it for this account instead of registering. */
export function ShareClaimSignedIn({ token }: { token: string }): ReactElement {
  const nav = useNavigate()
  const qc = useQueryClient()
  const [outcome, setOutcome] = useState<Outcome>({ state: 'pending' })

  useEffect(() => {
    let live = true
    oncePerKey(`claim:${token}`, () => api.claimShareInvite(token)).then(
      () => {
        if (!live) return
        setOutcome({ state: 'done' })
        void qc.invalidateQueries({ queryKey: ['share-invites'] })
      },
      (e: unknown) => {
        if (live) setOutcome({ state: 'failed', message: e instanceof Error ? e.message : String(e) })
      }
    )
    return () => {
      live = false
    }
  }, [qc, token])

  if (outcome.state === 'pending') {
    return (
      <Center mih="70vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <AuthHeader
        title={outcome.state === 'done' ? 'Invite added' : 'Invite link'}
        subtitle={
          outcome.state === 'done'
            ? 'Accept it under Shared to add it to yours.'
            : 'You are signed in, so this link adds the invite to your account.'
        }
      />
      <Paper p="xl" radius="md" withBorder>
        <Stack gap="md">
          {outcome.state === 'failed' ? (
            <Alert color="red" variant="light">
              {outcome.message}
            </Alert>
          ) : null}
          <Button rightSection={<IconArrowRight size={16} stroke={1.5} />} onClick={() => void nav({ to: '/shared' })}>
            Open Shared
          </Button>
        </Stack>
      </Paper>
    </Stack>
  )
}
