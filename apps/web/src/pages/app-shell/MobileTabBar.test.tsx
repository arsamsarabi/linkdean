import type { ReactNode } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useShellUi } from '@/stores'
import { MobileTabBar } from './MobileTabBar'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, search, children, ...rest }: { to: string; search?: Record<string, unknown>; children: ReactNode }) => {
    const q = search
      ? new URLSearchParams(
          Object.entries(search)
            .filter(([, v]) => v !== undefined)
            .map(([k, v]) => [k, String(v)])
        ).toString()
      : ''
    return (
      <a href={q ? `${to}?${q}` : to} {...rest}>
        {children}
      </a>
    )
  },
}))

function renderBar(pathname = '/', folder?: string): void {
  render(
    <MantineProvider env="test">
      <MobileTabBar pathname={pathname} folder={folder} />
    </MantineProvider>
  )
}

describe('MobileTabBar', () => {
  afterEach(cleanup)

  it('links Home, Library and Me, and marks the open one', () => {
    renderBar('/library')
    expect(screen.getByRole('link', { name: 'Home' }).getAttribute('href')).toBe('/')
    expect(screen.getByRole('link', { name: 'Library' }).hasAttribute('data-active')).toBe(true)
    expect(screen.getByRole('link', { name: 'Me' }).getAttribute('href')).toBe('/profile')
  })

  it('adds into the open folder', () => {
    renderBar('/library', 'f1')
    expect(screen.getByRole('link', { name: 'Add' }).getAttribute('href')).toBe('/library?folder=f1&create=true')
  })

  it('opens the command palette from Search', async () => {
    useShellUi.setState({ paletteOpen: false })
    renderBar()
    await userEvent.click(screen.getByRole('button', { name: 'Search' }))
    expect(useShellUi.getState().paletteOpen).toBe(true)
  })
})
