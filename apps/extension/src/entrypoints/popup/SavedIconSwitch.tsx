import { useEffect, useState, type ReactElement } from 'react'
import { Switch } from '@mantine/core'
import { browser } from 'wxt/browser'
import { requestTabsPermission, savedIconItem, savedIconOfferedItem } from '@/lib/savedPages'

/** Opt-in: colour the toolbar icon on pages already saved, grey elsewhere. */
export function SavedIconSwitch(): ReactElement {
  const [on, setOn] = useState(false)

  useEffect(() => {
    const sync = async (): Promise<void> => {
      const [wanted, granted] = await Promise.all([
        savedIconItem.getValue(),
        browser.permissions.contains({ permissions: ['tabs'] }),
      ])
      setOn(wanted && granted)
    }
    void sync()
    return savedIconItem.watch(() => void sync())
  }, [])

  function change(next: boolean): void {
    void savedIconOfferedItem.setValue(true)
    if (next) {
      // Ask straight from the click: browsers allow the prompt only from a user gesture.
      void requestTabsPermission().then((ok) => (ok ? savedIconItem.setValue(true) : undefined))
    } else {
      void savedIconItem.setValue(false)
      void browser.permissions.remove({ permissions: ['tabs'] })
    }
  }

  return (
    <Switch
      size="xs"
      checked={on}
      onChange={(e) => change(e.currentTarget.checked)}
      label="Color the icon on saved pages"
      description="Grey on pages you have not saved. Asks to read your tabs' addresses; they are checked in the browser and never sent."
    />
  )
}
