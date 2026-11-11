import { type ReactElement } from 'react'
import { Alert, Anchor, Badge, Button, Code, Group, Paper, SimpleGrid, Stack, Text } from '@mantine/core'
import { IconMail, IconSend } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components'
import { useEmailStatus, useSendTestEmail } from '@/hooks'

const SETUP_GUIDE = 'https://github.com/arsamsarabi/linkdean/blob/main/docs/self-hosting/operations.md#email'

function Stat({ label, value }: { label: string; value: number }): ReactElement {
  return (
    <Stack gap={0}>
      <Text size="xl" fw={700}>
        {value}
      </Text>
      <Text size="xs" c="dimmed">
        {label}
      </Text>
    </Stack>
  )
}

export function AdminEmailPanel(): ReactElement {
  const status = useEmailStatus()
  const test = useSendTestEmail()

  if (status.isError) {
    return <QueryErrorAlert error={status.error} onRetry={() => void status.refetch()} title="Email status failed" />
  }

  return (
    <Paper p="xl" withBorder>
      <Stack gap="md">
        <Group justify="space-between">
          <Group gap="xs">
            <IconMail size={18} stroke={1.5} />
            <Text fw={500}>Email</Text>
          </Group>
          {status.data ? (
            <Badge color={status.data.enabled ? 'green' : 'gray'} variant="light">
              {status.data.enabled ? 'On' : 'Off'}
            </Badge>
          ) : null}
        </Group>

        {!status.data ? (
          <Text size="sm" c="dimmed">
            Loading email status…
          </Text>
        ) : status.data.enabled && status.data.config ? (
          <>
            <Text size="sm" c="dimmed">
              Sending as {status.data.config.from} through {status.data.config.host}:{status.data.config.port} (
              {status.data.config.tls}).
            </Text>
            <SimpleGrid cols={3}>
              <Stat label="Sent" value={status.data.stats.sent} />
              <Stat label="Queued" value={status.data.stats.queued} />
              <Stat label="Failed" value={status.data.stats.failed} />
            </SimpleGrid>
            <Text size="xs" c="dimmed">
              Sent and failed cover the last 24 hours. Queued is every email still waiting.
            </Text>
            {status.data.stats.last_error ? (
              <Alert color="red" title="Latest error">
                {status.data.stats.last_error}
                {status.data.stats.last_error_at
                  ? ` · ${new Date(status.data.stats.last_error_at).toLocaleString()}`
                  : ''}
              </Alert>
            ) : null}
            <Group>
              <Button
                leftSection={<IconSend size={16} stroke={1.5} />}
                loading={test.isPending}
                onClick={() => test.mutate()}
              >
                Send test email
              </Button>
            </Group>
            {test.data?.ok ? <Alert color="green">Test email sent to {test.data.to}.</Alert> : null}
            {test.data && !test.data.ok ? (
              <Alert color="red" title="Test email failed">
                {test.data.error}
              </Alert>
            ) : null}
            {test.error ? <Alert color="red">{test.error.message}</Alert> : null}
          </>
        ) : (
          <Text size="sm" c="dimmed">
            Mail is off. Set <Code>SMTP_HOST</Code> and <Code>MAIL_FROM</Code> (usually with <Code>SMTP_USERNAME</Code>{' '}
            and <Code>SMTP_PASSWORD</Code>), then restart Linkdean.{' '}
            <Anchor href={SETUP_GUIDE} target="_blank" rel="noreferrer">
              Email setup guide
            </Anchor>
          </Text>
        )}
      </Stack>
    </Paper>
  )
}
