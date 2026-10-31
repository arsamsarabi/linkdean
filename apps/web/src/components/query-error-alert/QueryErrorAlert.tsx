import type { ReactElement } from 'react'
import { Alert, Button, Group, Stack, Text } from '@mantine/core'
import { IconAlertCircle, IconRefresh } from '@tabler/icons-react'

type Props = {
  error: unknown
  onRetry: () => void
  title?: string
}

export function QueryErrorAlert({ error, onRetry, title = 'Something went wrong' }: Props): ReactElement {
  const message = error instanceof Error ? error.message : String(error)
  return (
    <Alert color="red" variant="light" icon={<IconAlertCircle size={16} />} title={title} withCloseButton={false}>
      <Stack gap="sm">
        <Text size="sm">{message}</Text>
        <Group justify="flex-end">
          <Button size="xs" variant="light" leftSection={<IconRefresh size={14} stroke={1.5} />} onClick={onRetry}>
            Retry
          </Button>
        </Group>
      </Stack>
    </Alert>
  )
}
