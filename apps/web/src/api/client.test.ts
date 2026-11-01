import { afterEach, describe, expect, it, vi } from 'vitest'
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
