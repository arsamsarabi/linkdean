import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { AdminPlatformSettings } from './AdminPlatformSettings'

const state = vi.hoisted(() => ({ mail: false, require: false, save: vi.fn() }))

vi.mock('@/hooks', () => ({
  usePlatformSettings: () => ({
    isError: false,
    data: {
      settings: {
        registration_open: true,
        api_keys_enabled: true,
        require_verified_email: state.require,
        updated_at: '2026-01-01T00:00:00Z',
      },
      require_verified_email_forced: false,
    },
  }),
  useUpdatePlatformSettings: () => ({ isPending: false, mutate: state.save }),
  useReindexSearch: () => ({ isPending: false, mutate: vi.fn() }),
  useBootstrap: () => ({ data: { mail_enabled: state.mail } }),
}))
vi.mock('./AdminEmailPanel', () => ({ AdminEmailPanel: () => null }))
vi.mock('./AdminInvitesPanel', () => ({ AdminInvitesPanel: () => null }))
vi.mock('./AdminReportsPanel', () => ({ AdminReportsPanel: () => null }))
vi.mock('./AdminUsersPanel', () => ({ AdminUsersPanel: () => null }))
vi.mock('./BannedUsersPanel', () => ({ BannedUsersPanel: () => null }))

function renderPanel(): void {
  render(
    <MantineProvider>
      <AdminPlatformSettings />
    </MantineProvider>
  )
}

describe('AdminPlatformSettings require verified email', () => {
  beforeEach(() => {
    state.mail = false
    state.require = false
    state.save = vi.fn()
  })
  afterEach(cleanup)

  it('is disabled while mail is off', () => {
    renderPanel()
    expect((screen.getByRole('switch', { name: /^Require verified email/ }) as HTMLInputElement).disabled).toBe(true)
  })

  it('turns on when mail is on', async () => {
    state.mail = true
    renderPanel()
    await userEvent.click(screen.getByRole('switch', { name: /^Require verified email/ }))
    expect(state.save).toHaveBeenCalledWith({ require_verified_email: true }, expect.anything())
  })

  it('can still be turned off with mail off', () => {
    state.require = true
    renderPanel()
    expect((screen.getByRole('switch', { name: /^Require verified email/ }) as HTMLInputElement).disabled).toBe(false)
  })
})
