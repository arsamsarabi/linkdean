import { beforeEach, describe, expect, it } from 'vitest'
import { canPeekBookmark, usePeekUi } from './peekUi'

describe('peekUi', () => {
  beforeEach(() => {
    localStorage.clear()
    usePeekUi.setState({ target: null, mode: 'open' })
  })

  it('opens and closes a target', () => {
    usePeekUi.getState().open({
      bookmarkId: 'b1',
      url: 'https://example.com',
      title: 'Ex',
    })
    expect(usePeekUi.getState().target?.bookmarkId).toBe('b1')
    usePeekUi.getState().close()
    expect(usePeekUi.getState().target).toBeNull()
  })

  it('persists mode in localStorage', () => {
    usePeekUi.getState().maximize()
    expect(localStorage.getItem('linkdean-peek-mode')).toBe('maximized')
    usePeekUi.getState().minimize()
    expect(localStorage.getItem('linkdean-peek-mode')).toBe('minimized')
  })

  it('canPeekBookmark excludes vault and non-https', () => {
    expect(canPeekBookmark({ url: 'https://a.com', is_vault: false })).toBe(true)
    expect(canPeekBookmark({ url: 'https://a.com', is_vault: true })).toBe(false)
    expect(canPeekBookmark({ url: 'http://a.com', is_vault: false })).toBe(false)
  })
})
