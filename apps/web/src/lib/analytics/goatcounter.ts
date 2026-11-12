import type { AnyRouter } from '@tanstack/react-router'

/**
 * Page-view counting with the instance's GoatCounter, when the operator sets `GOATCOUNTER_URL`.
 *
 * Only the route's name is sent (`/library`, `/bookmarks/$id`), never ids, search terms, titles
 * or the referrer. Nothing is stored in the browser and no script is loaded from GoatCounter:
 * one `sendBeacon` per page. Browsers that send Do Not Track or Global Privacy Control are not
 * counted at all.
 */

/** A route id such as `/_app/bookmarks/$id` as the path GoatCounter shows: `/bookmarks/$id`. */
export function routePath(routeId: string): string {
  const parts = routeId.split('/').filter((p) => p && !p.startsWith('_'))
  return `/${parts.join('/')}`
}

type Screen = { width: number; height: number; devicePixelRatio: number }

/** The count URL. The host prefix keeps the app apart from the marketing site and docs. */
export function beaconUrl(origin: string, host: string, path: string, screen: Screen, rnd: string): string {
  const page = `${host}${path}`
  const params = new URLSearchParams({
    p: page,
    t: page,
    s: `${screen.width},${screen.height},${screen.devicePixelRatio || 1}`,
    rnd,
  })
  return `${origin}/count?${params}`
}

type Privacy = { doNotTrack?: string | null; globalPrivacyControl?: boolean }

/** Do Not Track or Global Privacy Control: count nothing. */
export function optedOut(nav: Privacy): boolean {
  return nav.doNotTrack === '1' || nav.globalPrivacyControl === true
}

async function goatcounterOrigin(): Promise<string | null> {
  try {
    const res = await fetch('/api/auth/config', { credentials: 'same-origin' })
    if (!res.ok) return null
    const body = (await res.json()) as { goatcounter_url?: unknown }
    return typeof body.goatcounter_url === 'string' && body.goatcounter_url ? body.goatcounter_url : null
  } catch {
    return null
  }
}

/** Count each page the router lands on, once per route, if the instance has analytics on. */
export async function startPageCounting(router: AnyRouter): Promise<void> {
  if (optedOut(navigator as Privacy) || typeof navigator.sendBeacon !== 'function') return
  const origin = await goatcounterOrigin()
  if (!origin) return
  let last = ''
  const count = (): void => {
    const match = router.state.matches.at(-1)
    if (!match) return
    const path = routePath(match.routeId)
    if (path === last) return
    last = path
    const { width, height } = window.screen
    const screen = { width, height, devicePixelRatio: window.devicePixelRatio }
    navigator.sendBeacon(beaconUrl(origin, location.host, path, screen, Math.random().toString(36).slice(2)))
  }
  router.subscribe('onResolved', count)
  count()
}
