import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Paper, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import { IconPuzzle } from '@tabler/icons-react'
import { useSearch } from '@tanstack/react-router'
import { api } from '@/api'
import { useMe } from '@/hooks'

/** The message the extension's script on this page listens for. */
export const EXTENSION_CODE_MESSAGE = 'linkdean-extension-code'

export type ConnectExtensionSearch = {
  extension_origin?: string
  code_challenge?: string
  code_challenge_method?: string
  state?: string
}

/**
 * The browser extension's sign-in window lands here. Allow posts a single-use code into this
 * window, where a script the extension placed on this page picks it up; the extension trades
 * it for its own key and closes the window. The code is useless without the verifier only the
 * extension holds.
 */
export function ConnectExtensionPage(): ReactElement {
  const search: ConnectExtensionSearch = useSearch({ from: '/_app/connect/extension' })
  const me = useMe(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [cancelled, setCancelled] = useState(false)
  const [connected, setConnected] = useState(false)
  const complete = !!search.extension_origin && !!search.code_challenge

  async function allow(): Promise<void> {
    if (!search.extension_origin || !search.code_challenge) return
    setBusy(true)
    setError(null)
    try {
      const { code, state } = await api.authorizeExtension({
        extension_origin: search.extension_origin,
        code_challenge: search.code_challenge,
        code_challenge_method: search.code_challenge_method ?? 'S256',
        state: search.state,
      })
      // Same window, same origin only: other sites and frames never see it. Also left on the
      // page, in case the extension's script arrives after the click.
      const message = { type: EXTENSION_CODE_MESSAGE, code, state }
      document.documentElement.dataset.linkdeanExtensionCode = JSON.stringify(message)
      window.postMessage(message, window.location.origin)
      setConnected(true)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  function cancel(): void {
    setCancelled(true)
    window.close()
  }

  return (
    <Stack maw={480} mx="auto" mt="xl" px="md">
      <Paper p="xl" withBorder>
        <Stack gap="md">
          <Group gap="sm" wrap="nowrap">
            <ThemeIcon variant="light" color="violet" size="lg">
              <IconPuzzle size={20} />
            </ThemeIcon>
            <Title order={3}>Connect the browser extension</Title>
          </Group>
          {!complete ? (
            <Alert color="yellow" title="Start from the extension">
              Open the Linkdean extension and choose <b>Sign in</b>. It brings you here with everything this page needs.
            </Alert>
          ) : cancelled ? (
            <Text>Cancelled. You can close this window.</Text>
          ) : connected ? (
            <Text>Connected. This window closes by itself; if it does not, close it and open the extension again.</Text>
          ) : (
            <>
              <Text>
                The Linkdean extension wants to use your account
                {me.data?.user.email ? (
                  <>
                    {' '}
                    <b>{me.data.user.email}</b>
                  </>
                ) : null}
                . It will be able to save bookmarks and read your folders, tags and bookmarks.
              </Text>
              <Text size="sm" c="dimmed">
                Only allow this if you just chose <b>Sign in</b> in the Linkdean extension. You can disconnect it any
                time under <b>Settings → API keys</b>, or by signing out in the extension.
              </Text>
              {error ? (
                <Alert color="red" role="alert">
                  {error}
                </Alert>
              ) : null}
              <Group justify="flex-end">
                <Button variant="default" onClick={cancel} disabled={busy}>
                  Cancel
                </Button>
                <Button onClick={() => void allow()} loading={busy}>
                  Allow
                </Button>
              </Group>
            </>
          )}
        </Stack>
      </Paper>
    </Stack>
  )
}
