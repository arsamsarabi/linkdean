import { type ReactElement } from 'react'
import { Alert, Anchor, Button, Center, Loader, Paper, Stack, Text } from '@mantine/core'
import { IconMailOff } from '@tabler/icons-react'
import { useUnsubscribe, useUnsubscribeLink } from '@/hooks'
import motion from '@/styles/motion.module.css'
import { AuthHeader } from '../password-reset/AuthHeader'

/** The signed part of the link (`u`, `s`, `t`), passed back to the API as it came. */
function linkQuery(): string {
  const params = new URLSearchParams(window.location.search)
  const out = new URLSearchParams()
  for (const key of ['u', 's', 't']) out.set(key, params.get(key)?.trim() ?? '')
  return out.toString()
}

function Frame({ title, subtitle, children }: { title: string; subtitle: string; children: ReactElement }) {
  return (
    <Stack maw={420} mx="auto" mt={80} px="md" pb={48} gap="lg" className={motion.fadeIn}>
      <AuthHeader title={title} subtitle={subtitle} />
      <Paper p="xl" radius="md" withBorder>
        {children}
      </Paper>
    </Stack>
  )
}

const settingsLink = (
  <Anchor href="/settings?tab=notifications" size="sm">
    Settings → Notifications
  </Anchor>
)

/**
 * `/unsubscribe?u=…&s=…&t=…`, linked from every notification email. It asks before turning
 * anything off, because link scanners open every URL in an email. Works signed out.
 */
export function UnsubscribePage(): ReactElement {
  const query = linkQuery()
  const link = useUnsubscribeLink(query)
  const unsubscribe = useUnsubscribe()

  if (link.isPending) {
    return (
      <Center mih="70vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  if (link.isError) {
    return (
      <Frame title="Unsubscribe" subtitle="Stop notification email.">
        <Stack gap="md">
          <Alert color="red" variant="light" title="Link not valid">
            This unsubscribe link is not valid. Copy the whole link from the email, or change your email settings in
            Linkdean.
          </Alert>
          {settingsLink}
        </Stack>
      </Frame>
    )
  }

  const digest = link.data.scope === 'digest'
  const what = digest ? 'the daily digest' : `email for ${link.data.label}`

  if (unsubscribe.isSuccess) {
    return (
      <Frame title="Unsubscribed" subtitle={`You will not get ${what} any more.`}>
        <Stack gap="sm">
          <Text size="sm">
            {digest ? 'Every kind that came in the digest is now off.' : 'Other notification email is not affected.'}{' '}
            You still see these notifications in Linkdean.
          </Text>
          <Text size="sm">Turn it back on any time in {settingsLink}.</Text>
        </Stack>
      </Frame>
    )
  }

  return (
    <Frame title="Unsubscribe" subtitle={`Stop ${what}?`}>
      <Stack gap="md">
        <Text size="sm">
          {digest
            ? 'This turns off email for every kind you get in the daily digest.'
            : `This turns off email for ${link.data.label}.`}{' '}
          You can turn it back on in Settings.
        </Text>
        {unsubscribe.isError ? (
          <Alert color="red" variant="light">
            {unsubscribe.error.message}
          </Alert>
        ) : null}
        <Button
          leftSection={<IconMailOff size={16} stroke={1.5} />}
          loading={unsubscribe.isPending}
          onClick={() => unsubscribe.mutate(query)}
        >
          Unsubscribe
        </Button>
      </Stack>
    </Frame>
  )
}
