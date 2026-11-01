import { type ReactElement } from 'react'
import { Anchor, Button, Group, Paper, Stack, Text, TextInput, Title } from '@mantine/core'
import { IconLogin } from '@tabler/icons-react'
import motion from '@/styles/motion.module.css'

type Props = {
  inviteCode: string
  onInviteCode: (v: string) => void
  onContinue: () => void
  onLogin: () => void
}

export function RegisterInviteGate({ inviteCode, onInviteCode, onContinue, onLogin }: Props): ReactElement {
  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <Stack gap={6}>
        <Title order={1} c="violet">
          Registration closed
        </Title>
        <Text c="dimmed">This instance is invite-only. Ask an admin for an invite link, or paste a code below.</Text>
      </Stack>
      <Paper p="xl" radius="md" withBorder>
        <Stack gap="md">
          <TextInput
            label="Invite code"
            placeholder="ldi_…"
            value={inviteCode}
            onChange={(e) => onInviteCode(e.currentTarget.value)}
            data-autofocus
          />
          <Button variant="light" disabled={!inviteCode.trim()} onClick={onContinue}>
            Continue with invite
          </Button>
          <Anchor component="button" type="button" onClick={onLogin}>
            <Group gap={6} justify="center">
              <IconLogin size={14} stroke={1.5} />
              Back to sign in
            </Group>
          </Anchor>
        </Stack>
      </Paper>
    </Stack>
  )
}
