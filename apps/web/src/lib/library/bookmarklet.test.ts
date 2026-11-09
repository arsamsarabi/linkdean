import { describe, expect, it, vi } from 'vitest'
import { bookmarkletSource } from './bookmarklet'

/** Runs the bookmarklet against a fake page and returns what it opened or navigated to. */
function run(opts: { selection?: string; popupBlocked?: boolean }): { opened?: string; navigated?: string } {
  const src = bookmarkletSource('https://app.example/')
  expect(src.startsWith('javascript:')).toBe(true)
  const out: { opened?: string; navigated?: string } = {}
  const fakeWindow = {
    getSelection: () => opts.selection ?? '',
    open: vi.fn((url: string) => {
      out.opened = url
      return opts.popupBlocked ? null : {}
    }),
  }
  const fakeLocation = {
    href: 'https://news.example/a b?x=1&y=2',
    set navigate(url: string) {
      out.navigated = url
    },
  }
  // `location.href = …` in the bookmarklet becomes a setter we can observe.
  const code = src.slice('javascript:'.length).replace('location.href=b+q', 'location.navigate=b+q')
  new Function('window', 'location', 'document', code)(fakeWindow, fakeLocation, { title: 'A & B' })
  return out
}

describe('bookmarklet', () => {
  it('opens quick save in a popup with the page address, title and selection', () => {
    const { opened, navigated } = run({ selection: '  quoted text ' })
    expect(navigated).toBeUndefined()
    const url = new URL(opened ?? '')
    expect(url.origin + url.pathname).toBe('https://app.example/save')
    expect(url.searchParams.get('popup')).toBe('1')
    expect(url.searchParams.get('url')).toBe('https://news.example/a b?x=1&y=2')
    expect(url.searchParams.get('title')).toBe('A & B')
    expect(url.searchParams.get('text')).toBe('quoted text')
  })

  it('leaves out empty selections and falls back to the same tab when popups are blocked', () => {
    const { navigated } = run({ popupBlocked: true })
    const url = new URL(navigated ?? '')
    expect(url.searchParams.has('text')).toBe(false)
    expect(url.searchParams.has('popup')).toBe(false)
    expect(url.searchParams.get('title')).toBe('A & B')
  })
})
