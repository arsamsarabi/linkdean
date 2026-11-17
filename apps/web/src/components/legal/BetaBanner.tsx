import { useState, type ReactElement } from 'react'
import { Alert, Anchor } from '@mantine/core'
import { useBootstrap } from '@/hooks'

export const BETA_BANNER_KEY = 'linkdean.betaBanner.dismissed'

function wasDismissed(): boolean {
  try {
    return sessionStorage.getItem(BETA_BANNER_KEY) === '1'
  } catch {
    // Storage blocked (private mode, policy): show the banner every load.
    return false
  }
}

/**
 * Public-beta notice above every page, on a server with its own terms (the hosted service).
 * Dismissal lasts for the browser session.
 */
export function BetaBanner(): ReactElement | null {
  const [hidden, setHidden] = useState<boolean>(wasDismissed)
  const legal = useBootstrap().data
  if (hidden || !legal?.terms_url) return null
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
      <Anchor href={legal.terms_url} inherit>
        Terms
      </Anchor>
      {legal.privacy_url ? (
        <>
          {' '}
          and{' '}
          <Anchor href={legal.privacy_url} inherit>
            Privacy Policy
          </Anchor>
        </>
      ) : null}
      .
    </Alert>
  )
}
