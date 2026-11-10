import { browser } from 'wxt/browser'
import { defineBackground } from 'wxt/utils/define-background'
import { createBookmark, savableUrl } from '@/lib/api'
import { sessionItem } from '@/lib/session'
import { signIn } from '@/lib/signIn'

export type SignInMessage = { type: 'sign-in'; instance: string }
export type SignInReply = { ok: true } | { ok: false; error: string }

const MENU_PAGE = 'save-page'
const MENU_LINK = 'save-link'

/** A short mark on the toolbar button: ✓ saved, ! failed (the button's tooltip says why). */
async function flash(tabId: number | undefined, ok: boolean, title: string): Promise<void> {
  await browser.action.setBadgeBackgroundColor({ color: ok ? '#2f9e44' : '#e03131', tabId })
  await browser.action.setBadgeText({ text: ok ? '✓' : '!', tabId })
  await browser.action.setTitle({ title, tabId })
  setTimeout(() => {
    void browser.action.setBadgeText({ text: '', tabId })
    void browser.action.setTitle({ title: 'Save to Linkdean', tabId })
  }, 4000)
}

export default defineBackground(() => {
  browser.runtime.onInstalled.addListener(() => {
    void browser.contextMenus.removeAll().then(() => {
      browser.contextMenus.create({ id: MENU_PAGE, title: 'Save page to Linkdean (Unsorted)', contexts: ['page'] })
      browser.contextMenus.create({ id: MENU_LINK, title: 'Save link to Linkdean (Unsorted)', contexts: ['link'] })
    })
  })

  browser.contextMenus.onClicked.addListener((info, tab) => {
    void (async () => {
      const url = info.menuItemId === MENU_LINK ? info.linkUrl : tab?.url
      const title = info.menuItemId === MENU_PAGE ? tab?.title : undefined
      const session = await sessionItem.getValue()
      if (!session) return flash(tab?.id, false, 'Sign in to Linkdean first: click this button.')
      if (!savableUrl(url)) return flash(tab?.id, false, 'Linkdean saves only http and https pages.')
      try {
        await createBookmark(session, { url, title, category_ids: [], tags: [] })
        await flash(tab?.id, true, 'Saved to Unsorted in Linkdean')
      } catch (e) {
        await flash(tab?.id, false, e instanceof Error ? e.message : String(e))
      }
    })()
  })

  // Sign-in runs here, not in the popup: the popup closes when the sign-in window takes focus.
  browser.runtime.onMessage.addListener((message: unknown, _sender, sendResponse: (r: SignInReply) => void) => {
    const msg = message as SignInMessage
    if (msg?.type !== 'sign-in') return false
    signIn(msg.instance).then(
      () => sendResponse({ ok: true }),
      (e: unknown) => sendResponse({ ok: false, error: e instanceof Error ? e.message : String(e) })
    )
    return true
  })
})
