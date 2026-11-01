import { type ReactElement } from 'react'
import { Alert, Button, Group, Paper, PasswordInput, Stack, ThemeIcon, Title } from '@mantine/core'
import { IconInfoCircle, IconKey } from '@tabler/icons-react'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  hasPassword: boolean
  currentPw: string
  newPw: string
  newPw2: string
  saving: boolean
  onCurrentPw: (v: string) => void
  onNewPw: (v: string) => void
  onNewPw2: (v: string) => void
  onSubmit: () => void
}

export function ProfilePasswordCard({
  hasPassword,
  currentPw,
  newPw,
  newPw2,
  saving,
  onCurrentPw,
  onNewPw,
  onNewPw2,
  onSubmit,
}: Props): ReactElement {
  if (!hasPassword) {
    return (
      <Alert icon={<IconInfoCircle size={16} />} color="violet" variant="light">
        OIDC account — no local password to change.
      </Alert>
    )
  }

  return (
    <Paper p="xl" withBorder>
      <Stack gap="lg">
        <Group gap="sm">
          <ThemeIcon size={28} radius="md" variant="light" color="violet">
            <IconKey size={16} stroke={1.5} />
          </ThemeIcon>
          <Title order={4}>Change password</Title>
        </Group>
        <PasswordInput
          label="Current password"
          value={currentPw}
          onChange={(e) => onCurrentPw(e.currentTarget.value)}
        />
        <PasswordInput
          label="New password"
          description="Min 8 characters"
          value={newPw}
          onChange={(e) => onNewPw(e.currentTarget.value)}
        />
        <PasswordInput label="Confirm new password" value={newPw2} onChange={(e) => onNewPw2(e.currentTarget.value)} />
        <Button
          leftSection={<IconKey {...iconSm} />}
          loading={saving}
          disabled={newPw.length < 8 || newPw !== newPw2 || !currentPw}
          onClick={onSubmit}
        >
          Update password
        </Button>
      </Stack>
    </Paper>
  )
}
