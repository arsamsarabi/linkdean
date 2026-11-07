import { useState, type ReactElement } from 'react'
import { Alert, Anchor } from '@mantine/core'

export const BETA_BANNER_KEY = 'linkdean.betaBanner.dismissed'

function wasDismissed(): boolean {
  try {
    return sessionStorage.getItem(BETA_BANNER_KEY) === '1'
  } catch {
    // Storage blocked (private mode, policy): show the banner every load.
    return false
  }
}

/** Public-beta notice above every page. Dismissal lasts for the browser session. */
export function BetaBanner(): ReactElement | null {
  const [hidden, setHidden] = useState<boolean>(wasDismissed)
  if (hidden) return null
  return (
    <Alert
      color="yellow"
      variant="light"
      radius={0}
      withCloseButton
      closeButtonLabel="Dismiss beta notice"
      onClose={() => {
        try {
          sessionStorage.setItem(BETA_BANNER_KEY, '1')
        } catch {
          // Dismiss for this page view only.
        }
        setHidden(true)
      }}
    >
      Linkdean is in public beta: no uptime promise, so export your data regularly. See the{' '}
      <Anchor href="/terms" inherit>
        Terms
      </Anchor>{' '}
      and{' '}
      <Anchor href="/privacy" inherit>
        Privacy Policy
      </Anchor>
      .
    </Alert>
  )
}
