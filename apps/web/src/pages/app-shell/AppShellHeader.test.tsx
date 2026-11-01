import type { ReactNode } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { AppShellHeader } from './AppShellHeader'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, children, ...rest }: { to: string; children: ReactNode }) => (
    <a href={to} {...rest}>
      {children}
    </a>
  ),
}))

vi.mock('@/components', () => ({
  BrokenLinksButton: () => null,
  DuplicatesButton: () => null,
  HistoryButton: () => null,
  LogoMark: () => null,
  NotificationsBell: () => null,
  PageContainer: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  ThemeMenuSwitch: () => null,
  UserAvatar: () => null,
}))

function renderHeader(vaultUnlocked: boolean) {
  return render(
    <MantineProvider env="test">
      <AppShellHeader
        loading={false}
        landing="/"
        onDashboard={false}
        onLibrary
        onVaults={false}
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

  it('links to Vaults while a vault is open', () => {
    renderHeader(true)
    expect(screen.getByRole('link', { name: 'Vaults' }).getAttribute('href')).toBe('/vaults')
  })

  it('hides the link when every vault is locked', () => {
    renderHeader(false)
    expect(screen.queryByRole('link', { name: 'Vaults' })).toBeNull()
  })

  it('keeps vault entries out of the user menu', async () => {
    renderHeader(true)
    await userEvent.click(screen.getByRole('button', { name: 'Account menu' }))
    expect(await screen.findByRole('menuitem', { name: 'Profile' })).toBeTruthy()
    expect(screen.queryByRole('menuitem', { name: /vault|unlock/i })).toBeNull()
  })
})
