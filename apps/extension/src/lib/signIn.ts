import { browser } from 'wxt/browser'
import { storage } from 'wxt/utils/storage'
import { z } from 'zod'
import { originPattern } from './instance'
import { challengeFor, randomState, randomVerifier } from './pkce'
import { lastInstanceItem, sessionItem, type Session } from './session'

/** The message the approve page posts, and the one the relay script passes on. */
export const PAGE_MESSAGE = 'linkdean-extension-code'
export const RELAY_MESSAGE = 'extension-code'

/** A sign-in waiting for the person to approve it in the Linkdean window. */
export type PendingSignIn = {
  instance: string
  verifier: string
  state: string
  extensionOrigin: string
  windowId: number
  tabId: number
}

// Session storage: survives the background worker being stopped while the person signs in,
// and is gone when the browser closes.
export const pendingItem = storage.defineItem<PendingSignIn | null>('session:pendingSignIn', { fallback: null })
/** Why the last sign-in failed, shown the next time the popup opens. */
export const signInErrorItem = storage.defineItem<string | null>('local:signInError', { fallback: null })

/**
 * A sign-in the popup asked for. Chrome closes the popup while it asks for the site permission,
 * so the popup records the request first and the background starts it once the grant arrives.
 */
export type SignInIntent = { instance: string; at: number }
export const signInIntentItem = storage.defineItem<SignInIntent | null>('session:signInIntent', { fallback: null })
/** An intent older than this is stale: the person walked away from the permission prompt. */
export const SIGN_IN_INTENT_MAX_AGE_MS = 2 * 60_000

/** Whether `intent` is recent and for one of the `origins` patterns just granted or asked about. */
export function intentMatches(intent: SignInIntent | null, origins: string[], now: number): intent is SignInIntent {
  return (
    intent !== null && now - intent.at < SIGN_IN_INTENT_MAX_AGE_MS && origins.includes(originPattern(intent.instance))
  )
}

let taking: Promise<unknown> = Promise.resolve()

/**
 * Hands out the recorded sign-in once and clears it. The permission grant and the popup (when
 * it stayed open) both ask; taking one at a time means only one of them opens a window.
 */
export function takeSignInIntent(origins: string[]): Promise<string | null> {
  const run = taking.then(async () => {
    const intent = await signInIntentItem.getValue()
    if (!intentMatches(intent, origins, Date.now())) return null
    await signInIntentItem.setValue(null)
    return intent.instance
  })
  taking = run.catch(() => undefined)
  return run
}

/** The approve page on `instance` for this request. */
export function connectUrl(instance: string, extensionOrigin: string, challenge: string, state: string): string {
  const q = new URLSearchParams({
    extension_origin: extensionOrigin,
    code_challenge: challenge,
    code_challenge_method: 'S256',
    state,
  })
  return `${instance}/connect/extension?${q.toString()}`
}

/**
 * Fails with a readable reason when `instance` cannot sign the extension in: not reachable,
 * not Linkdean, or a Linkdean from before the extension. An empty trade is refused by a new
 * enough instance (400 or 422) and is a 404 on an older one.
 */
export async function checkInstance(instance: string): Promise<void> {
  let res: Response
  try {
    res = await fetch(`${instance}/api/auth/extension/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: '{}',
    })
  } catch {
    throw new Error(`Could not reach ${new URL(instance).host}. Check the address and your connection.`)
  }
  if (res.status === 404 || res.status === 405) {
    throw new Error(`${new URL(instance).host} does not support the extension yet. It needs a newer Linkdean.`)
  }
}

/** Opens the approve page in a window of its own, centred on the window in use. */
export async function startSignIn(instance: string): Promise<void> {
  await checkInstance(instance)
  await signInErrorItem.setValue(null)
  const verifier = randomVerifier()
  const state = randomState()
  const extensionOrigin = browser.runtime.getURL('/')
  const url = connectUrl(instance, extensionOrigin, await challengeFor(verifier), state)
  const width = 480
  const height = 720
  const current = await browser.windows.getLastFocused().catch(() => undefined)
  const left =
    current?.left !== undefined && current.width !== undefined
      ? Math.round(current.left + (current.width - width) / 2)
      : undefined
  const top =
    current?.top !== undefined && current.height !== undefined
      ? Math.round(current.top + Math.max(0, (current.height - height) / 3))
      : undefined
  const win = await browser.windows.create({ url, type: 'popup', width, height, left, top })
  const tabId = win?.tabs?.[0]?.id
  if (win?.id === undefined || tabId === undefined) throw new Error('Could not open the sign-in window.')
  await pendingItem.setValue({ instance, verifier, state, extensionOrigin, windowId: win.id, tabId })
}

/**
 * Runs inside the approve page (as an isolated content script) and hands the code the page
 * posts to the extension. Self-contained: it is serialised into the page.
 */
export function relayCode(pageMessage: string, relayMessage: string): void {
  const flag = '__linkdeanExtensionRelay'
  const w = window as unknown as Record<string, unknown>
  if (w[flag]) return
  w[flag] = true
  const send = (data: unknown): void => {
    if (!data || typeof data !== 'object') return
    const d = data as { type?: unknown; code?: unknown; state?: unknown }
    if (d.type !== pageMessage || typeof d.code !== 'string') return
    // Chrome exposes `chrome`, Firefox both; this runs outside the bundle, so no polyfill.
    const g = globalThis as unknown as {
      chrome?: { runtime: { sendMessage: (m: unknown) => Promise<unknown> } }
      browser?: { runtime: { sendMessage: (m: unknown) => Promise<unknown> } }
    }
    void (g.browser ?? g.chrome)?.runtime.sendMessage({ type: relayMessage, code: d.code, state: d.state })
  }
  window.addEventListener('message', (e) => {
    if (e.source === window && e.origin === window.location.origin) send(e.data)
  })
  const left = document.documentElement.dataset.linkdeanExtensionCode
  if (left) {
    try {
      send(JSON.parse(left))
    } catch {
      // Not ours.
    }
  }
}

/** Places the relay on the sign-in window's page once it is on the instance. */
export async function injectRelay(tabId: number, tabUrl: string | undefined): Promise<void> {
  const pending = await pendingItem.getValue()
  if (!pending || pending.tabId !== tabId || !tabUrl?.startsWith(`${pending.instance}/`)) return
  await browser.scripting
    .executeScript({ target: { tabId }, func: relayCode, args: [PAGE_MESSAGE, RELAY_MESSAGE] })
    .catch(() => undefined)
}

const tokenSchema = z.object({ token: z.string(), key_id: z.string(), email: z.string() })

/** Trades the code from the sign-in window for a key, then closes the window. */
export async function finishSignIn(
  code: string,
  state: unknown,
  fromTabId: number | undefined
): Promise<Session | null> {
  const pending = await pendingItem.getValue()
  if (!pending || pending.tabId !== fromTabId || pending.state !== state) return null
  await pendingItem.setValue(null)
  try {
    const res = await fetch(`${pending.instance}/api/auth/extension/token`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code, code_verifier: pending.verifier, extension_origin: pending.extensionOrigin }),
    })
    const body: unknown = await res.json().catch(() => null)
    if (!res.ok) {
      const msg =
        body && typeof body === 'object' && 'error' in body ? String(body.error) : `Linkdean answered ${res.status}.`
      throw new Error(msg)
    }
    const t = tokenSchema.parse(body)
    const session: Session = { instance: pending.instance, token: t.token, keyId: t.key_id, email: t.email }
    await sessionItem.setValue(session)
    await lastInstanceItem.setValue(pending.instance)
    return session
  } catch (e) {
    await signInErrorItem.setValue(e instanceof Error ? e.message : String(e))
    return null
  } finally {
    await browser.windows.remove(pending.windowId).catch(() => undefined)
  }
}

/** The person closed the sign-in window before approving. */
export async function abandonSignIn(windowId: number): Promise<void> {
  const pending = await pendingItem.getValue()
  if (pending?.windowId === windowId) await pendingItem.setValue(null)
}
