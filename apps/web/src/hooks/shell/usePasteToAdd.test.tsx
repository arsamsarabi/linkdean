import { cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { usePasteToAdd } from './usePasteToAdd'

const nav = vi.hoisted(() => vi.fn())
const location = vi.hoisted(() => ({ pathname: '/library', search: { folder: 'f1' } as Record<string, unknown> }))

vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => nav,
  useRouterState: ({ select }: { select: (s: { location: typeof location }) => unknown }) => select({ location }),
}))

function paste(text: string, target: EventTarget = document.body): void {
  const e = new Event('paste', { bubbles: true, cancelable: true }) as ClipboardEvent
  Object.defineProperty(e, 'clipboardData', { value: { getData: () => text } })
  target.dispatchEvent(e)
}

describe('usePasteToAdd', () => {
  afterEach(() => {
    cleanup()
    nav.mockClear()
    document.body.innerHTML = ''
  })

  it('opens the new-bookmark form for a pasted address, in the open folder', () => {
    renderHook(() => usePasteToAdd(true))
    paste('https://react.dev/learn')
    expect(nav).toHaveBeenCalledWith({
      to: '/library',
      search: expect.objectContaining({ addUrl: 'https://react.dev/learn', folder: 'f1' }),
    })
  })

  it('leaves pastes into text fields, dialogs and plain text alone', () => {
    renderHook(() => usePasteToAdd(true))
    const input = document.createElement('input')
    document.body.appendChild(input)
    input.focus()
    paste('https://react.dev', input)
    input.blur()
    paste('just some words')
    const dialog = document.createElement('div')
    dialog.setAttribute('role', 'dialog')
    document.body.appendChild(dialog)
    paste('https://react.dev')
    expect(nav).not.toHaveBeenCalled()
  })

  it('does nothing until enabled', () => {
    renderHook(() => usePasteToAdd(false))
    paste('https://react.dev')
    expect(nav).not.toHaveBeenCalled()
  })
})
