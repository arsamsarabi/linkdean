import { afterEach, describe, expect, it, vi } from 'vitest'
import { isSignedIn, markSignedIn } from '@/lib'
import { banFromBody, req } from './client'

function respond(status: number, body: unknown): void {
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(new Response(JSON.stringify(body), { status })))
}

const banned = { error: 'banned', code: 'banned', ban: { reason: 'Spam', ends_at: null } }

describe('banned responses', () => {
  afterEach(() => vi.unstubAllGlobals())

  function stubLocation(pathname: string) {
    const assign = vi.fn()
    vi.stubGlobal('location', { pathname, assign })
    return assign
  }

  it('reads the ban from a body', () => {
    expect(banFromBody(banned)).toEqual({ reason: 'Spam', ends_at: null })
    expect(banFromBody({ error: 'forbidden' })).toBeNull()
    expect(banFromBody(null)).toBeNull()
  })

  it('sends a banned 403 to the banned page', async () => {
    const assign = stubLocation('/library')
    respond(403, banned)
    await expect(req('/bookmarks')).rejects.toThrow('banned')
    expect(assign).toHaveBeenCalledWith('/banned')
  })

  it('stays put on the banned page, so its own check cannot loop', async () => {
    const assign = stubLocation('/banned')
    respond(403, banned)
    await expect(req('/auth/me')).rejects.toThrow()
    expect(assign).not.toHaveBeenCalled()
  })

  it('ignores any other 403', async () => {
    const assign = stubLocation('/library')
    respond(403, { error: 'forbidden' })
    await expect(req('/admin/users')).rejects.toThrow('forbidden')
    expect(assign).not.toHaveBeenCalled()
  })
})

describe('credentials', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('sends the cookie and the client header, never a bearer token', async () => {
    const fetchMock = vi.fn().mockResolvedValue(new Response('{}', { status: 200 }))
    vi.stubGlobal('fetch', fetchMock)
    await req('/bookmarks', { method: 'POST', body: '{}' })
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit
    const headers = new Headers(init.headers)
    expect(init.credentials).toBe('include')
    expect(headers.get('X-Linkdean-Client')).toBe('web')
    expect(headers.has('Authorization')).toBe(false)
  })
})

describe('expired sessions', () => {
  afterEach(() => {
    vi.unstubAllGlobals()
    markSignedIn(false)
  })

  function stubLocation(pathname: string) {
    const assign = vi.fn()
    vi.stubGlobal('location', { pathname, assign })
    return assign
  }

  const expired = { error: 'session expired', code: 'session_expired' }

  it('signs out and goes to login', async () => {
    markSignedIn(true)
    const assign = stubLocation('/library')
    respond(401, expired)
    await expect(req('/bookmarks')).rejects.toThrow('session expired')
    expect(isSignedIn()).toBe(false)
    expect(assign).toHaveBeenCalledWith('/login')
  })

  it('keeps the session on a wrong password', async () => {
    markSignedIn(true)
    const assign = stubLocation('/settings')
    respond(401, { error: 'unauthorized' })
    await expect(req('/users/me/password')).rejects.toThrow('unauthorized')
    expect(isSignedIn()).toBe(true)
    expect(assign).not.toHaveBeenCalled()
  })

  it('leaves signed-out visitors where they are', async () => {
    const assign = stubLocation('/l/some-list')
    respond(401, expired)
    await expect(req('/auth/me')).rejects.toThrow()
    expect(assign).not.toHaveBeenCalled()
  })

  it('sends a cookie session (OIDC) to login from an app page', async () => {
    const assign = stubLocation('/library')
    respond(401, expired)
    await expect(req('/bookmarks')).rejects.toThrow('session expired')
    expect(assign).toHaveBeenCalledWith('/login')
  })

  it('does not loop on the login page', async () => {
    markSignedIn(true)
    const assign = stubLocation('/login')
    respond(401, expired)
    await expect(req('/auth/me')).rejects.toThrow()
    expect(isSignedIn()).toBe(false)
    expect(assign).not.toHaveBeenCalled()
  })
})
