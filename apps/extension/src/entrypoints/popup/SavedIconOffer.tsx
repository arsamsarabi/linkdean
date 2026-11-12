import { useEffect, useState, type ReactElement } from 'react'
import { Button, Group, Paper, Stack, Text } from '@mantine/core'
import { requestTabsPermission, savedIconItem, savedIconOfferedItem, tabsPermissionHeld } from '@/lib/savedPages'

/**
 * Asked once, the first time the popup opens signed in: turn on the saved-page icon. The
 * permission stays optional, so installing shows no extra warning. The switch below the save
 * form changes it later.
 */
export function SavedIconOffer(): ReactElement | null {
  const [show, setShow] = useState(false)

  useEffect(() => {
    void Promise.all([savedIconOfferedItem.getValue(), savedIconItem.getValue(), tabsPermissionHeld()]).then(
      ([offered, on, held]) => setShow(!offered && !(on && held))
    )
  }, [])

  function allow(): void {
    // Chrome may close this popup while it asks, so record the answer first. The background
    // turns the icon on when the grant arrives.
    void savedIconOfferedItem.setValue(true)
    // Ask straight from the click: browsers allow the prompt only from a user gesture.
    void requestTabsPermission().then((ok) => (ok ? savedIconItem.setValue(true) : undefined))
    setShow(false)
  }

  function notNow(): void {
    void savedIconOfferedItem.setValue(true)
    setShow(false)
  }

  if (!show) return null
  return (
    <Paper withBorder p="sm" mb="md">
      <Stack gap="xs">
        <Text size="sm" fw={600}>
          Show which pages you have saved?
        </Text>
        <Text size="xs" c="dimmed">
          The toolbar icon turns grey on pages that are not in your library. Your browser asks to let the extension read
          your tabs&apos; addresses; they are checked here and never sent.
        </Text>
        <Group gap="xs">
          <Button size="xs" onClick={allow}>
            Allow
          </Button>
          <Button size="xs" variant="default" onClick={notNow}>
            Not now
          </Button>
        </Group>
      </Stack>
    </Paper>
  )
}
