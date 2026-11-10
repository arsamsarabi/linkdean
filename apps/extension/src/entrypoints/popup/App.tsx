import { useEffect, useState, type ReactElement } from 'react'
import { Anchor, Box, Group, Text } from '@mantine/core'
import { revokeOwnKey } from '@/lib/api'
import { sessionItem, type Session } from '@/lib/session'
import { SaveView } from './SaveView'
import { SignInView } from './SignInView'

export function App(): ReactElement | null {
  const [session, setSession] = useState<Session | null | undefined>(undefined)

  useEffect(() => {
    void sessionItem.getValue().then(setSession)
    return sessionItem.watch(setSession)
  }, [])

  async function signOut(current: Session): Promise<void> {
    // Revoke on the server first; sign out here even if that fails (offline, already revoked).
    await revokeOwnKey(current).catch(() => undefined)
    await sessionItem.setValue(null)
  }

  if (session === undefined) return null
  return (
    <Box w={380} p="md">
      {session ? (
        <>
          <SaveView session={session} />
          <Group justify="space-between" mt="md" gap="xs" wrap="nowrap">
            <Text size="xs" c="dimmed" truncate>
              {session.email} · {new URL(session.instance).host}
            </Text>
            <Anchor component="button" size="xs" onClick={() => void signOut(session)}>
              Sign out
            </Anchor>
          </Group>
        </>
      ) : (
        <SignInView />
      )}
    </Box>
  )
}
