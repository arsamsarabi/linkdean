import { useEffect, useEffectEvent, useId, type ReactElement } from 'react'
import { Text } from '@mantine/core'

declare global {
  interface Window {
    turnstile?: {
      render: (
        el: HTMLElement,
        opts: {
          sitekey: string
          callback: (token: string) => void
          'expired-callback'?: () => void
          'error-callback'?: () => void
        }
      ) => string
      remove: (widgetId: string) => void
      reset: (widgetId: string) => void
    }
    onTurnstileApiLoad?: () => void
  }
}

const SCRIPT_ID = 'cf-turnstile-api'

function loadTurnstileScript(): Promise<void> {
  if (window.turnstile) return Promise.resolve()
  const existing = document.getElementById(SCRIPT_ID)
  if (existing) {
    return new Promise((resolve) => {
      const prev = window.onTurnstileApiLoad
      window.onTurnstileApiLoad = () => {
        prev?.()
        resolve()
      }
    })
  }
  return new Promise((resolve, reject) => {
    window.onTurnstileApiLoad = () => resolve()
    const script = document.createElement('script')
    script.id = SCRIPT_ID
    script.src = 'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit&onload=onTurnstileApiLoad'
    script.async = true
    script.onerror = () => reject(new Error('Failed to load captcha'))
    document.head.appendChild(script)
  })
}

type Props = {
  siteKey: string
  onToken: (token: string | null) => void
}

/** Cloudflare Turnstile widget. Parent clears token after a failed submit and may remount to reset. */
export function TurnstileWidget({ siteKey, onToken }: Props): ReactElement {
  const hostId = useId()
  // Turnstile calls back long after render; this always reaches the latest `onToken`.
  const emitToken = useEffectEvent((token: string | null) => onToken(token))

  useEffect(() => {
    // One object per mount, so the async render and the cleanup share state without
    // reassigning locals after the effect has run.
    const mount: { cancelled: boolean; widgetId: string | null } = { cancelled: false, widgetId: null }
    void (async () => {
      try {
        await loadTurnstileScript()
        if (mount.cancelled || !window.turnstile) return
        const el = document.getElementById(hostId)
        if (!el) return
        mount.widgetId = window.turnstile.render(el, {
          sitekey: siteKey,
          callback: (token) => emitToken(token),
          'expired-callback': () => emitToken(null),
          'error-callback': () => emitToken(null),
        })
      } catch {
        emitToken(null)
      }
    })()
    return () => {
      mount.cancelled = true
      if (mount.widgetId && window.turnstile) {
        window.turnstile.remove(mount.widgetId)
      }
    }
  }, [hostId, siteKey])

  return (
    <div>
      <div id={hostId} />
      <Text size="xs" c="dimmed" mt={4}>
        Protected by Cloudflare Turnstile
      </Text>
    </div>
  )
}
