import { browser } from 'wxt/browser'
import { storage } from 'wxt/utils/storage'
import { z } from 'zod'
import { call, savableUrl } from './api'
import { sessionItem, type Session } from './session'

/**
 * The toolbar icon shows whether the open page is already saved: in colour when it is, grey
 * when it is not. Off by default; turning it on asks for the `tabs` permission so the
 * extension can read each tab's address.
 *
 * Checks happen here, in the browser. The extension downloads short hashes of the saved
 * addresses and compares each page against them, so pages someone only visits never reach the
 * server.
 */
export const savedIconItem = storage.defineItem<boolean>('local:savedIcon', { fallback: false })

type HashCache = { instance: string; hashes: string[]; fetchedAt: number }
const hashCacheItem = storage.defineItem<HashCache | null>('local:savedHashes', { fallback: null })

/** How long a downloaded list is used before it is fetched again. */
export const HASHES_MAX_AGE_MS = 5 * 60 * 1000
/** Hex digits of SHA-256 the server sends; must match `URL_HASH_HEX_LEN` in the API. */
const HEX_LEN = 16

const TABS: { permissions: ['tabs'] } = { permissions: ['tabs'] }

const COLOUR = { 16: '/icon/16.png', 32: '/icon/32.png', 48: '/icon/48.png', 128: '/icon/128.png' }
const GREY = {
  16: '/icon-grey/16.png',
  32: '/icon-grey/32.png',
  48: '/icon-grey/48.png',
  128: '/icon-grey/128.png',
}
export const TITLE_DEFAULT = 'Save to Linkdean'
export const TITLE_SAVED = 'Saved in Linkdean'

/** The first 16 hex digits of SHA-256 over the exact address, as the server computes them. */
export async function urlHash(url: string): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(url.trim()))
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, '0'))
    .join('')
    .slice(0, HEX_LEN)
}

/** Whether a cached list can still be used for this instance. */
export function cacheFresh(cache: HashCache | null, instance: string, now: number): cache is HashCache {
  return !!cache && cache.instance === instance && now - cache.fetchedAt < HASHES_MAX_AGE_MS
}

const hashesSchema = z.object({ hashes: z.array(z.string()) })

async function savedHashes(session: Session): Promise<Set<string>> {
  const cache = await hashCacheItem.getValue()
  if (cacheFresh(cache, session.instance, Date.now())) return new Set(cache.hashes)
  const { hashes } = await call(session, '/bookmarks/url-hashes', hashesSchema)
  await hashCacheItem.setValue({ instance: session.instance, hashes, fetchedAt: Date.now() })
  return new Set(hashes)
}

/** Record a page saved from the extension, so its icon turns at once. */
export async function rememberSaved(url: string): Promise<void> {
  const [cache, session] = await Promise.all([hashCacheItem.getValue(), sessionItem.getValue()])
  if (!cache || !session || cache.instance !== session.instance) return
  const hash = await urlHash(url)
  if (!cache.hashes.includes(hash)) await hashCacheItem.setValue({ ...cache, hashes: [...cache.hashes, hash] })
}

export async function forgetSaved(): Promise<void> {
  await hashCacheItem.setValue(null)
}

/** Whether the icon is on and allowed: the setting, the permission and a session. */
export async function savedIconActive(): Promise<Session | null> {
  const [on, session] = await Promise.all([savedIconItem.getValue(), sessionItem.getValue()])
  if (!on || !session) return null
  return (await browser.permissions.contains(TABS)) ? session : null
}

export function requestTabsPermission(): Promise<boolean> {
  return browser.permissions.request(TABS)
}

/** Paint one tab's icon and tooltip for the page it shows. */
export async function paintTab(tabId: number, url: string | undefined): Promise<void> {
  const session = await savedIconActive()
  if (!session) {
    await browser.action.setIcon({ tabId, path: COLOUR })
    await browser.action.setTitle({ tabId, title: TITLE_DEFAULT })
    return
  }
  let saved = false
  if (savableUrl(url)) {
    try {
      saved = (await savedHashes(session)).has(await urlHash(url))
    } catch {
      // Offline or signed out: show the page as not saved rather than guess.
    }
  }
  await browser.action.setIcon({ tabId, path: saved ? COLOUR : GREY })
  await browser.action.setTitle({ tabId, title: saved ? TITLE_SAVED : TITLE_DEFAULT })
}

/** Repaint every open tab, after the setting, the session or the permission changes. */
export async function paintAllTabs(): Promise<void> {
  const tabs = await browser.tabs.query({})
  await Promise.all(tabs.map((t) => (t.id === undefined ? undefined : paintTab(t.id, t.url).catch(() => undefined))))
}
