import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Paper, PasswordInput, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  useChangeGlobalPassphrase,
  useDisableGlobalPassphrase,
  useEnableGlobalPassphrase,
  useGlobalPassphrase,
} from '@/hooks'
import { UnlockAllVaultsModal } from './UnlockAllVaultsModal'

export function GlobalPassphraseCard(): ReactElement {
  const state = useGlobalPassphrase()
  const enable = useEnableGlobalPassphrase()
  const change = useChangeGlobalPassphrase()
  const disable = useDisableGlobalPassphrase()
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const enabled = !!state.data?.enabled
  const [unlockAllOpen, setUnlockAllOpen] = useState(false)

  async function run(action: Promise<unknown>, message: string): Promise<void> {
    try {
      await action
      notifications.show({ message })
      setCurrent('')
      setNext('')
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Paper p="lg" withBorder>
      <Stack>
        <Title order={4}>Global passphrase</Title>
        <Text size="sm" c="dimmed">
          Optional. Opens the vaults you add to it in one step. Each vault keeps its own passphrase.
        </Text>
        <Alert color="yellow" variant="light">
          Anyone who learns this passphrase can open every vault added to it.
        </Alert>
        {enabled ? (
          <PasswordInput
            label="Current passphrase"
            value={current}
            onChange={(e) => setCurrent(e.currentTarget.value)}
          />
        ) : null}
        <PasswordInput
          label={enabled ? 'New passphrase' : 'Passphrase'}
          value={next}
          onChange={(e) => setNext(e.currentTarget.value)}
        />
        <Group>
          {enabled ? (
            <>
              <Button variant="light" onClick={() => setUnlockAllOpen(true)}>
                Unlock all
              </Button>
              <Button
                disabled={next.length < 8 || !current}
                loading={change.isPending}
                onClick={() =>
                  void run(
                    change.mutateAsync({ current_passphrase: current, new_passphrase: next }),
                    'Global passphrase changed'
                  )
                }
              >
                Change
              </Button>
              <Button
                variant="default"
                loading={disable.isPending}
                onClick={() => void run(disable.mutateAsync(), 'Global passphrase turned off')}
              >
                Turn off
              </Button>
            </>
          ) : (
            <Button
              disabled={next.length < 8}
              loading={enable.isPending}
              onClick={() => void run(enable.mutateAsync(next), 'Global passphrase turned on')}
            >
              Turn on
            </Button>
          )}
        </Group>
      </Stack>
      {unlockAllOpen ? <UnlockAllVaultsModal onClose={() => setUnlockAllOpen(false)} /> : null}
    </Paper>
  )
}
