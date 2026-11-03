import { type ReactElement } from 'react'
import { Alert, Paper, Stack, Switch, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { QueryErrorAlert } from '@/components'
import { useBootstrap, useNotificationEmailPrefs, useSetNotificationEmailPref } from '@/hooks'
import { SettingsSkeleton } from './SettingsSkeleton'

/** Settings → Notifications: one "email me" switch per notification kind. */
export function NotificationEmailsPanel(): ReactElement {
  const bootstrap = useBootstrap()
  const mailEnabled = bootstrap.data?.mail_enabled ?? false
  const prefs = useNotificationEmailPrefs()
  const save = useSetNotificationEmailPref()

  if (prefs.isError) {
    return (
      <QueryErrorAlert error={prefs.error} onRetry={() => void prefs.refetch()} title="Notification settings failed" />
    )
  }

  if (!prefs.data) {
    return <SettingsSkeleton />
  }

  return (
    <Paper p="xl" withBorder maw={640}>
      <Stack gap="lg">
        <Stack gap={4}>
          <Title order={4}>Email me about</Title>
          <Text size="sm" c="dimmed">
            You always see these in Linkdean. Pick the ones you also want by email.
          </Text>
        </Stack>
        {bootstrap.data && !mailEnabled ? (
          <Alert color="gray">Email notifications need outbound email. Ask your admin.</Alert>
        ) : null}
        {prefs.data.prefs.map((pref) => (
          <Switch
            key={pref.kind}
            label={pref.label}
            description={pref.default ? 'On by default' : undefined}
            checked={pref.enabled}
            disabled={!mailEnabled}
            onChange={(e) => {
              const enabled = e.currentTarget.checked
              save.mutate(
                { kind: pref.kind, enabled },
                { onError: (err) => notifications.show({ color: 'red', message: String(err) }) }
              )
            }}
          />
        ))}
      </Stack>
    </Paper>
  )
}
