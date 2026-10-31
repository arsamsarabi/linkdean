import type { ReactElement } from 'react'
import { Alert, Button } from '@mantine/core'
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
      {message}
      <Button mt="sm" size="xs" variant="light" leftSection={<IconRefresh size={14} stroke={1.5} />} onClick={onRetry}>
        Retry
      </Button>
    </Alert>
  )
}
