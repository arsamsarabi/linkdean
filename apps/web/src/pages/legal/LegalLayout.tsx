import { type ReactElement, type ReactNode } from 'react'
import { Anchor, Group, Stack, Text, Title } from '@mantine/core'
import { TERMS_VERSION } from '@/lib'

type Props = {
  title: string
  children: ReactNode
}

/** Shared frame for the signed-out legal pages. */
export function LegalLayout({ title, children }: Props): ReactElement {
  return (
    <Stack maw={720} mx="auto" mt={48} px="md" pb={64} gap="lg" component="main">
      <Stack gap={4}>
        <Title order={1} c="violet">
          {title}
        </Title>
        <Text size="sm" c="dimmed">
          Version {TERMS_VERSION}. Draft for the public beta; it has not been reviewed by a lawyer and may change.
        </Text>
      </Stack>
      {children}
      <Group gap="md" component="nav" aria-label="Legal pages">
        <Anchor href="/terms" size="sm">
          Terms of Service
        </Anchor>
        <Anchor href="/privacy" size="sm">
          Privacy Policy
        </Anchor>
        <Anchor href="/acceptable-use" size="sm">
          Acceptable Use Policy
        </Anchor>
        <Anchor href="/login" size="sm">
          Back to sign in
        </Anchor>
      </Group>
    </Stack>
  )
}

type SectionProps = {
  heading: string
  children: ReactNode
}

export function LegalSection({ heading, children }: SectionProps): ReactElement {
  return (
    <Stack gap="xs" component="section">
      <Title order={2} size="h4">
        {heading}
      </Title>
      {children}
    </Stack>
  )
}
