import { type ReactElement } from 'react'
import { Button, Paper, Stack, Switch, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconRefresh } from '@tabler/icons-react'
import { QueryErrorAlert, TwoColumnLayout } from '@/components'
import { usePlatformSettings, useReindexSearch, useUpdatePlatformSettings } from '@/hooks'
import { AdminInvitesPanel } from './AdminInvitesPanel'
import { AdminUsersPanel } from './AdminUsersPanel'
import { SettingsSkeleton } from './SettingsSkeleton'

export function AdminPlatformSettings(): ReactElement {
  const platform = usePlatformSettings(true)
  const save = useUpdatePlatformSettings()
  const reindex = useReindexSearch()

  if (platform.isError) {
    return (
      <QueryErrorAlert
        error={platform.error}
        onRetry={() => void platform.refetch()}
        title="Platform settings failed"
      />
    )
  }

  if (!platform.data) {
    return <SettingsSkeleton />
  }

  const open = platform.data.settings.registration_open
  const apiKeysOn = platform.data.settings.api_keys_enabled

  return (
    <TwoColumnLayout
      left={
        <>
          <Paper p="xl" withBorder>
            <Stack gap="lg">
              <Switch
                label="Open registration"
                description="When off, only existing users can sign in. First-install bootstrap still works."
                checked={open}
                disabled={save.isPending}
                onChange={(e) => {
                  const registration_open = e.currentTarget.checked
                  save.mutate(
                    { registration_open },
                    {
                      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }}
              />
              <Switch
                label="API keys enabled"
                description="When off, no user can create or authenticate with API keys."
                checked={apiKeysOn}
                disabled={save.isPending}
                onChange={(e) => {
                  const api_keys_enabled = e.currentTarget.checked
                  save.mutate(
                    { api_keys_enabled },
                    {
                      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }}
              />
              {save.isPending ? (
                <Text size="xs" c="dimmed">
                  Saving…
                </Text>
              ) : null}
            </Stack>
          </Paper>
          <Paper p="xl" withBorder>
            <Stack gap="md">
              <Text fw={500}>Search index</Text>
              <Text size="sm" c="dimmed">
                Rebuild Meilisearch from Postgres (non-vault bookmarks). Clears stale docs first.
              </Text>
              <Button
                leftSection={<IconRefresh size={16} stroke={1.5} />}
                loading={reindex.isPending}
                onClick={() => {
                  reindex.mutate(undefined, {
                    onSuccess: (res) =>
                      notifications.show({
                        message: `Reindex done · ${res.upserted} upserted${res.failed ? ` · ${res.failed} failed` : ''}`,
                      }),
                    onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                  })
                }}
              >
                Reindex search
              </Button>
            </Stack>
          </Paper>
          <AdminInvitesPanel />
        </>
      }
      right={<AdminUsersPanel />}
    />
  )
}
