import { MantineProvider } from '@mantine/core'
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { usePeekUi } from '@/stores'
import { PEEK_IFRAME_SANDBOX, PeekPanel } from './PeekPanel'

vi.mock('@mantine/hooks', async (orig) => {
  const actual = await orig<typeof import('@mantine/hooks')>()
  return {
    ...actual,
    useHotkeys: (pairs: [string, () => void][]) => {
      for (const [key, fn] of pairs) {
        if (key === 'Escape') {
          ;(globalThis as { __peekEsc?: () => void }).__peekEsc = fn
        }
      }
    },
  }
})

function renderPeek() {
  return render(
    <MantineProvider env="test">
      <PeekPanel />
    </MantineProvider>
  )
}

const target = {
  bookmarkId: 'b1',
  url: 'https://example.com/page',
  title: 'Example page',
  description: 'A description',
  previewUrl: null as string | null,
}

describe('PeekPanel', () => {
  beforeEach(() => {
    usePeekUi.setState({ target, mode: 'open' })
  })

  afterEach(() => {
    cleanup()
    usePeekUi.setState({ target: null, mode: 'open' })
    delete (globalThis as { __peekEsc?: () => void }).__peekEsc
    vi.useRealTimers()
  })

  it('renders a sandboxed iframe without same-origin or top navigation', () => {
    const { container } = renderPeek()
    const frame = container.querySelector('iframe')
    expect(frame).toBeTruthy()
    expect(frame?.getAttribute('sandbox')).toBe(PEEK_IFRAME_SANDBOX)
    expect(frame?.getAttribute('sandbox')).not.toContain('allow-same-origin')
    expect(frame?.getAttribute('sandbox')).not.toContain('allow-top-navigation')
    expect(frame?.getAttribute('src')).toBe('https://example.com/page')
  })

  it('closes on Escape and via the close control', async () => {
    const { unmount } = renderPeek()
    ;(globalThis as { __peekEsc?: () => void }).__peekEsc?.()
    expect(usePeekUi.getState().target).toBeNull()
    unmount()

    usePeekUi.getState().open(target)
    renderPeek()
    await userEvent.click(screen.getByRole('button', { name: 'Close peek' }))
    expect(usePeekUi.getState().target).toBeNull()
  })

  it('maximizes and minimizes', async () => {
    renderPeek()
    await userEvent.click(screen.getByRole('button', { name: 'Maximize peek' }))
    expect(usePeekUi.getState().mode).toBe('maximized')
    await userEvent.click(screen.getByRole('button', { name: 'Minimize peek' }))
    expect(usePeekUi.getState().mode).toBe('minimized')
  })

  it('shows the blocked-frame fallback with open in new tab', () => {
    const { container } = renderPeek()
    const frame = container.querySelector('iframe')
    expect(frame).toBeTruthy()
    Object.defineProperty(frame, 'contentWindow', {
      configurable: true,
      get: () => ({ location: { href: 'about:blank' } }),
    })
    fireEvent.load(frame!)
    const dialog = screen.getByRole('dialog', { name: /peek: example page/i })
    expect(within(dialog).getByRole('link', { name: /open in new tab/i })).toHaveAttribute(
      'href',
      'https://example.com/page'
    )
    expect(within(dialog).getByText('A description')).toBeTruthy()
    expect(container.querySelector('iframe')).toBeNull()
  })
})
