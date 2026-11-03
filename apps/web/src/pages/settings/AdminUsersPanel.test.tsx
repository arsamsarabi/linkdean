import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AdminUserSummary } from '@/schemas'
import { AdminUsersPanel } from './AdminUsersPanel'

const setVerified = vi.hoisted(() => ({ isPending: false, mutate: vi.fn() }))
const changeEmail = vi.hoisted(() => ({ isPending: false, isError: false, error: null, mutate: vi.fn() }))
const users = vi.hoisted(() => ({ list: [] as AdminUserSummary[] }))

vi.mock('@/hooks', () => ({
  useAdminUsers: () => ({ data: { users: users.list }, isLoading: false, isError: false }),
  useUpdateAdminUser: () => ({ isPending: false, mutate: vi.fn() }),
  useMe: () => ({ data: { user: { id: 'admin' } } }),
  useLiftBans: () => ({ isPending: false, mutate: vi.fn() }),
  useSetEmailVerified: () => setVerified,
  useAdminChangeEmail: () => changeEmail,
}))

function user(over: Partial<AdminUserSummary>): AdminUserSummary {
  return {
    id: 'u1',
    email: 'ada@example.com',
    display_name: 'Ada',
    is_admin: false,
    api_keys_enabled: true,
    created_at: '2026-01-01T00:00:00Z',
    banned: false,
    has_password: true,
    email_verified: true,
    ...over,
  }
}

function renderPanel(): void {
  render(
    <MantineProvider>
      <AdminUsersPanel />
    </MantineProvider>
  )
}

describe('AdminUsersPanel email verification', () => {
  beforeEach(() => {
    setVerified.mutate = vi.fn()
    changeEmail.mutate = vi.fn()
  })
  afterEach(cleanup)

  it('badges an unverified account and marks it verified', async () => {
    users.list = [user({ email_verified: false })]
    renderPanel()
    expect(screen.getByText('Unverified')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Mark verified' }))
    expect(setVerified.mutate).toHaveBeenCalledWith({ id: 'u1', verified: true }, expect.anything())
  })

  it('removes verification from a verified account', async () => {
    users.list = [user({ email_verified: true })]
    renderPanel()
    expect(screen.queryByText('Unverified')).toBeNull()
    await userEvent.click(screen.getByRole('button', { name: 'Remove verification' }))
    expect(setVerified.mutate).toHaveBeenCalledWith({ id: 'u1', verified: false }, expect.anything())
  })

  it('changes an email from a small modal', async () => {
    users.list = [user({})]
    renderPanel()
    await userEvent.click(screen.getByRole('button', { name: 'Change email' }))
    const dialog = await screen.findByRole('dialog')
    const input = within(dialog).getByLabelText('New email')
    await userEvent.clear(input)
    await userEvent.type(input, 'ada@new.example.com')
    await userEvent.click(within(dialog).getByRole('button', { name: 'Change email' }))
    expect(changeEmail.mutate).toHaveBeenCalledWith({ id: 'u1', email: 'ada@new.example.com' }, expect.anything())
  })
})
