import { useEffect, useState, type ReactElement } from 'react'
import { Alert, Button, Stack, Text, TextInput, Title } from '@mantine/core'
import { browser } from 'wxt/browser'
import { DEFAULT_INSTANCE, normalizeInstance, originPattern } from '@/lib/instance'
import { lastInstanceItem } from '@/lib/session'
import type { SignInMessage, SignInReply } from '../background'

export function SignInView(): ReactElement {
  const [instance, setInstance] = useState(DEFAULT_INSTANCE)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    void lastInstanceItem.getValue().then((last) => last && setInstance(last))
  }, [])

  async function start(): Promise<void> {
    setError(null)
    let origin: string
    try {
      origin = normalizeInstance(instance)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
      return
    }
    // Must be the first await: browsers only show this prompt straight after a click.
    const granted = await browser.permissions.request({ origins: [originPattern(origin)] })
    if (!granted) {
      setError(`The extension needs permission to reach ${new URL(origin).host} to save there.`)
      return
    }
    setBusy(true)
    const reply = (await browser.runtime.sendMessage({ type: 'sign-in', instance: origin } satisfies SignInMessage)) as
      SignInReply | undefined
    setBusy(false)
    if (reply && !reply.ok) setError(reply.error)
  }

  return (
    <Stack gap="sm">
      <Title order={4}>Sign in to Linkdean</Title>
      <Text size="sm" c="dimmed">
        Use linkdean.app, or type the address of your own Linkdean.
      </Text>
      <TextInput
        label="Linkdean address"
        value={instance}
        onChange={(e) => setInstance(e.currentTarget.value)}
        onKeyDown={(e) => e.key === 'Enter' && void start()}
      />
      {error ? (
        <Alert color="red" role="alert">
          {error}
        </Alert>
      ) : null}
      <Button onClick={() => void start()} loading={busy}>
        Sign in
      </Button>
      <Text size="xs" c="dimmed">
        A Linkdean window opens: sign in there and choose Allow. The extension gets its own key, which you can revoke
        under Settings → API keys.
      </Text>
    </Stack>
  )
}
