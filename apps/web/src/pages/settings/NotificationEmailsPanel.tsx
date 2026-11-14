import { type ReactElement } from 'react'
import { Alert, Group, Paper, SegmentedControl, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { QueryErrorAlert } from '@/components'
import { useBootstrap, useNotificationEmailPrefs, useSetNotificationEmailPref } from '@/hooks'
import type { EmailDelivery, NotificationEmailPref } from '@/schemas'
import { SettingsSkeleton } from './SettingsSkeleton'

type Choice = 'off' | EmailDelivery

function choiceOf(pref: NotificationEmailPref): Choice {
  return pref.enabled ? pref.delivery : 'off'
}

function choicesFor(pref: NotificationEmailPref): { value: Choice; label: string }[] {
  const choices: { value: Choice; label: string }[] = [
    { value: 'off', label: 'Off' },
    { value: 'instant', label: 'Each one' },
  ]
  if (pref.digest_allowed) choices.push({ value: 'daily', label: 'Daily digest' })
  return choices
}

/** Settings → Notifications: per kind, no email, one email each, or a daily digest. */
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
            You always see these in Linkdean. Pick the ones you also want by email. A daily digest collects them into
            one email a day, sent after 8:00 in your time zone.
          </Text>
        </Stack>
        {bootstrap.data && !mailEnabled ? (
          <Alert color="gray">Email notifications need outbound email. Ask your admin.</Alert>
        ) : null}
        {prefs.data.prefs.map((pref) => (
          <Group key={pref.kind} justify="space-between" gap="sm" wrap="wrap">
            <Stack gap={0}>
              <Text size="sm" fw={500}>
                {pref.label}
              </Text>
              {pref.default ? (
                <Text size="xs" c="dimmed">
                  On by default
                </Text>
              ) : null}
            </Stack>
            <SegmentedControl
              size="xs"
              aria-label={`Email for ${pref.label}`}
              value={choiceOf(pref)}
              data={choicesFor(pref)}
              disabled={!mailEnabled}
              onChange={(value) => {
                const choice = value as Choice
                const vars =
                  choice === 'off'
                    ? { kind: pref.kind, enabled: false }
                    : { kind: pref.kind, enabled: true, delivery: choice }
                save.mutate(vars, { onError: (err) => notifications.show({ color: 'red', message: String(err) }) })
              }}
            />
          </Group>
        ))}
      </Stack>
    </Paper>
  )
}
