import { useEffect, useState, type ReactElement } from 'react'
import { ActionIcon, Box, Group, Stack, Text, Tooltip } from '@mantine/core'
import { IconLogout } from '@tabler/icons-react'
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
            <Stack gap={0} miw={0}>
              {[session.email, new URL(session.instance).host].map((line) => (
                <Tooltip
                  key={line}
                  label={line}
                  openDelay={400}
                  multiline
                  maw={340}
                  style={{ overflowWrap: 'anywhere' }}
                >
                  <Text size="xs" c="dimmed" truncate>
                    {line}
                  </Text>
                </Tooltip>
              ))}
            </Stack>
            <Tooltip label="Sign out">
              <ActionIcon variant="subtle" color="red.6" aria-label="Sign out" onClick={() => void signOut(session)}>
                <IconLogout size={18} />
              </ActionIcon>
            </Tooltip>
          </Group>
        </>
      ) : (
        <SignInView />
      )}
    </Box>
  )
}
