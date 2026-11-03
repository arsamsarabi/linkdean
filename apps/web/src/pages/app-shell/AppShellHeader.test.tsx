import type { ReactNode } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useShellUi } from '@/stores'
import { AppShellHeader } from './AppShellHeader'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, children, ...rest }: { to: string; children: ReactNode }) => (
    <a href={to} {...rest}>
      {children}
    </a>
  ),
}))

vi.mock('@/components', () => ({
  LogoMark: () => null,
  NotificationsBell: () => null,
  ThemeMenuSwitch: () => null,
  UserAvatar: () => null,
}))

function renderHeader(vaultUnlocked: boolean) {
  return render(
    <MantineProvider env="test">
      <AppShellHeader
        loading={false}
        landing="/"
        label="Ana"
        displayName="Ana"
        email="ana@example.com"
        hasAvatar={false}
        vaultUnlocked={vaultUnlocked}
        unreadCount={0}
        onLogout={() => undefined}
      />
    </MantineProvider>
  )
}

describe('AppShellHeader', () => {
  afterEach(cleanup)

  it('opens the command palette from the search field', async () => {
    useShellUi.setState({ paletteOpen: false })
    renderHeader(false)
    await userEvent.click(screen.getAllByRole('button', { name: 'Search' })[0])
    expect(useShellUi.getState().paletteOpen).toBe(true)
  })

  it('leaves adding to the library page, the tab bar and the palette', () => {
    renderHeader(false)
    expect(screen.queryByRole('link', { name: /add/i })).toBeNull()
  })

  it('keeps vault entries out of the user menu', async () => {
    renderHeader(true)
    await userEvent.click(screen.getByRole('button', { name: 'Account menu' }))
    expect(await screen.findByRole('menuitem', { name: 'Profile' })).toBeTruthy()
    expect(screen.queryByRole('menuitem', { name: /vault|unlock/i })).toBeNull()
  })
})
