import { type ReactElement } from 'react'
import { Group, Stack, ThemeIcon, Title } from '@mantine/core'
import { IconShield } from '@tabler/icons-react'
import { Navigate } from '@tanstack/react-router'
import { QueryErrorAlert } from '@/components'
import { useMe } from '@/hooks'
import { AdminPlatformSettings } from './AdminPlatformSettings'

export function AdminPage(): ReactElement | null {
  const me = useMe()

  if (me.isError) return <QueryErrorAlert error={me.error} onRetry={() => void me.refetch()} />
  if (!me.data) return null
  if (!me.data.user.is_admin) return <Navigate to="/" replace />

  return (
    <Stack gap="xl">
      <Group gap="md">
        <ThemeIcon size={40} radius="md" variant="light" color="violet">
          <IconShield size={22} stroke={1.5} />
        </ThemeIcon>
        <Title order={2}>Admin</Title>
      </Group>
      <AdminPlatformSettings />
    </Stack>
  )
}
