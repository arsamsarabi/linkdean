import { afterEach, describe, expect, it, vi } from 'vitest'
import { checkInstance, connectUrl, PAGE_MESSAGE, RELAY_MESSAGE, relayCode } from './signIn'

const origin = `chrome-extension://${'a'.repeat(32)}/`

describe('sign-in', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    delete document.documentElement.dataset.linkdeanExtensionCode
    delete (window as unknown as Record<string, unknown>).__linkdeanExtensionRelay
  })

  it('sends the request to the instance approve page', () => {
    const url = new URL(connectUrl('https://app.linkdean.app', origin, 'chal', 'st'))
    expect(url.origin + url.pathname).toBe('https://app.linkdean.app/connect/extension')
    expect(Object.fromEntries(url.searchParams)).toEqual({
      extension_origin: origin,
      code_challenge: 'chal',
      code_challenge_method: 'S256',
      state: 'st',
    })
  })

  it('tells an older or unreachable instance apart before opening a window', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 404 })))
    await expect(checkInstance('https://old.example')).rejects.toThrow(/newer Linkdean/)
    vi.stubGlobal('fetch', vi.fn().mockRejectedValue(new TypeError('offline')))
    await expect(checkInstance('https://gone.example')).rejects.toThrow(/Could not reach gone.example/)
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(null, { status: 422 })))
    await expect(checkInstance('https://app.linkdean.app')).resolves.toBeUndefined()
  })

  it('relays the code the page posts, and one it left before the relay arrived', async () => {
    const sendMessage = vi.fn().mockResolvedValue(undefined)
    vi.stubGlobal('chrome', { runtime: { sendMessage } })
    document.documentElement.dataset.linkdeanExtensionCode = JSON.stringify({
      type: PAGE_MESSAGE,
      code: 'early',
      state: 's',
    })
    relayCode(PAGE_MESSAGE, RELAY_MESSAGE)
    relayCode(PAGE_MESSAGE, RELAY_MESSAGE) // a second injection adds no second listener
    expect(sendMessage).toHaveBeenCalledWith({ type: RELAY_MESSAGE, code: 'early', state: 's' })

    window.dispatchEvent(
      new MessageEvent('message', {
        data: { type: PAGE_MESSAGE, code: 'c1', state: 's' },
        origin: window.location.origin,
        source: window,
      })
    )
    window.dispatchEvent(
      new MessageEvent('message', {
        data: { type: 'other', code: 'x' },
        origin: window.location.origin,
        source: window,
      })
    )
    window.dispatchEvent(
      new MessageEvent('message', {
        data: { type: PAGE_MESSAGE, code: 'evil' },
        origin: 'https://evil.example',
        source: window,
      })
    )
    expect(sendMessage.mock.calls.map((c) => c[0].code)).toEqual(['early', 'c1'])
  })
})
