import { type ReactElement } from 'react'
import { Anchor, Group } from '@mantine/core'

/** Footer row for the signed-out pages. */
export function LegalFooterLinks(): ReactElement {
  return (
    <Group gap="md" justify="center">
      <Anchor href="/terms" size="xs" c="dimmed">
        Terms
      </Anchor>
      <Anchor href="/privacy" size="xs" c="dimmed">
        Privacy
      </Anchor>
    </Group>
  )
}
