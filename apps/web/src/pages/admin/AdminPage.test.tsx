import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { AdminPage } from './AdminPage'

const state = vi.hoisted(() => ({ tab: undefined as string | undefined, nav: vi.fn() }))

vi.mock('@tanstack/react-router', () => ({
  useSearch: () => ({ tab: state.tab }),
  useNavigate: () => state.nav,
  Navigate: () => null,
}))
vi.mock('@/hooks', () => ({
  useMe: () => ({ isError: false, data: { user: { is_admin: true } } }),
}))
vi.mock('./AdminPlatformSettings', () => ({
  AdminPlatformSettings: () => <p>general panel</p>,
  AdminQuotasPanel: () => <p>quotas panel</p>,
  AdminSearchIndexPanel: () => <p>search panel</p>,
}))
vi.mock('./AdminEmailPanel', () => ({ AdminEmailPanel: () => <p>email panel</p> }))
vi.mock('./AdminInvitesPanel', () => ({ AdminInvitesPanel: () => <p>invites panel</p> }))
vi.mock('./AdminReportsPanel', () => ({ AdminReportsPanel: () => <p>reports panel</p> }))
vi.mock('./AdminScreenshotRetryPanel', () => ({ AdminScreenshotRetryPanel: () => <p>screenshots panel</p> }))
vi.mock('./AdminUsersPanel', () => ({ AdminUsersPanel: () => <p>users panel</p> }))
vi.mock('./BannedUsersPanel', () => ({ BannedUsersPanel: () => <p>bans panel</p> }))

function renderPage(): void {
  render(
    <MantineProvider>
      <AdminPage />
    </MantineProvider>
  )
}

describe('AdminPage tabs', () => {
  beforeEach(() => {
    state.tab = undefined
    state.nav = vi.fn()
  })
  afterEach(cleanup)

  it('opens General by default and on an unknown tab', () => {
    state.tab = 'nope'
    renderPage()
    expect(screen.getByText('general panel')).toBeTruthy()
    expect(screen.queryByText('users panel')).toBeNull()
  })

  it('opens the tab named in the URL', () => {
    state.tab = 'screenshots'
    renderPage()
    expect(screen.getByText('screenshots panel')).toBeTruthy()
    expect(screen.queryByText('general panel')).toBeNull()
  })

  it('puts the chosen tab in the URL, and General back to a bare /admin', async () => {
    state.tab = 'users'
    renderPage()
    await userEvent.click(screen.getByRole('tab', { name: 'Reports' }))
    expect(state.nav).toHaveBeenCalledWith({ to: '/admin', search: { tab: 'reports' } })
    await userEvent.click(screen.getByRole('tab', { name: 'General' }))
    expect(state.nav).toHaveBeenCalledWith({ to: '/admin', search: {} })
  })
})
