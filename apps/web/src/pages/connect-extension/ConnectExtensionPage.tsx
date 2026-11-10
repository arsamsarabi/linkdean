import { useState, type ReactElement } from 'react'
import { Alert, Button, Group, Paper, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import { IconPuzzle } from '@tabler/icons-react'
import { useSearch } from '@tanstack/react-router'
import { api } from '@/api'
import { useMe } from '@/hooks'

export type ConnectExtensionSearch = {
  redirect_uri?: string
  code_challenge?: string
  code_challenge_method?: string
  state?: string
}

/**
 * The browser extension's sign-in window lands here. Allow sends the window back to the
 * extension with a single-use code, which the extension trades for its own key.
 */
export function ConnectExtensionPage(): ReactElement {
  const search: ConnectExtensionSearch = useSearch({ from: '/_app/connect/extension' })
  const me = useMe(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [cancelled, setCancelled] = useState(false)
  const complete = !!search.redirect_uri && !!search.code_challenge

  async function allow(): Promise<void> {
    if (!search.redirect_uri || !search.code_challenge) return
    setBusy(true)
    setError(null)
    try {
      const { redirect } = await api.authorizeExtension({
        redirect_uri: search.redirect_uri,
        code_challenge: search.code_challenge,
        code_challenge_method: search.code_challenge_method ?? 'S256',
        state: search.state,
      })
      // The browser hands this address to the extension and closes the window.
      window.location.assign(redirect)
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
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
