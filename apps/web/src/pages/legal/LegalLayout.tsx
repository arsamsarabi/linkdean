import { type ReactElement, type ReactNode } from 'react'
import { Anchor, Group, Stack, Text, Title } from '@mantine/core'
import { TERMS_VERSION } from '@/lib'

type Props = {
  title: string
  /** Which document the beta notice names, such as "These Terms" or "This Privacy Policy". */
  noticeSubject: string
  children: ReactNode
}

/** Shared frame for the signed-out legal pages. */
export function LegalLayout({ title, noticeSubject, children }: Props): ReactElement {
  return (
    <Stack maw={720} mx="auto" mt={48} px="md" pb={64} gap="lg" component="main">
      <Stack gap={4}>
        <Title order={1} c="violet">
          {title}
        </Title>
        <Text size="sm" fw={600}>
          Version: {TERMS_VERSION}
        </Text>
        <Text size="sm" c="dimmed">
          <strong>Public beta notice:</strong> Linkdean is currently in public beta. {noticeSubject} not been reviewed
          by a lawyer and may be updated as the service develops.
        </Text>
      </Stack>
      {children}
      <Group gap="md" wrap="wrap" component="nav" aria-label="Legal pages">
        <Anchor href="/terms" size="sm">
          Terms of Service
        </Anchor>
        <Anchor href="/privacy" size="sm">
          Privacy Policy
        </Anchor>
        <Anchor href="/login" size="sm">
          Back to sign in
        </Anchor>
      </Group>
    </Stack>
  )
}

/** An email address in legal copy, as a mailto link. */
export function MailLink({ address }: { address: string }): ReactElement {
  return (
    <Anchor href={`mailto:${address}`} inherit>
      {address}
    </Anchor>
  )
}

type SectionProps = {
  heading: string
  /** Anchor for deep links, such as `/terms#acceptable-use`. */
  id?: string
  children: ReactNode
}

export function LegalSection({ heading, id, children }: SectionProps): ReactElement {
  return (
    <Stack gap="xs" component="section" id={id} style={id ? { scrollMarginTop: 24 } : undefined}>
      <Title order={2} size="h4">
        {heading}
      </Title>
      {children}
    </Stack>
  )
}

/** A numbered part inside a section, such as 5.1. */
export function LegalSubsection({ heading, children }: Omit<SectionProps, 'id'>): ReactElement {
  return (
    <Stack gap="xs" component="section" pl="md">
      <Title order={3} size="h5">
        {heading}
      </Title>
      {children}
    </Stack>
  )
}
