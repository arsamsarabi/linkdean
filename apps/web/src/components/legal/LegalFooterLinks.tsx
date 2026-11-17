import { type ReactElement } from 'react'
import { Anchor, Group } from '@mantine/core'
import type { LegalLinks } from '@/schemas'

/** Footer row for the signed-out pages: the operator's legal pages, when set. */
export function LegalFooterLinks({ legal }: { legal: LegalLinks }): ReactElement | null {
  if (!legal.terms_url && !legal.privacy_url) return null
  return (
    <Group gap="md" justify="center">
      {legal.terms_url ? (
        <Anchor href={legal.terms_url} size="xs" c="dimmed">
          Terms
        </Anchor>
      ) : null}
      {legal.privacy_url ? (
        <Anchor href={legal.privacy_url} size="xs" c="dimmed">
          Privacy
        </Anchor>
      ) : null}
    </Group>
  )
}
