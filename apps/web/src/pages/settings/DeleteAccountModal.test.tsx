import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { DeleteAccountModal } from './DeleteAccountModal'

const schedule: { isPending: boolean; isError: boolean; error: Error | null; mutate: ReturnType<typeof vi.fn> } = {
  isPending: false,
  isError: false,
  error: null,
  mutate: vi.fn(),
}

vi.mock('@/hooks', () => ({ useScheduleAccountDeletion: () => schedule }))

function renderModal(hasPassword: boolean, reauthed = false): void {
  render(
    <MantineProvider>
      <DeleteAccountModal
        email="Me@Example.com"
        hasPassword={hasPassword}
        ssoProvider="auth.example.com"
        reauthed={reauthed}
        onClose={vi.fn()}
      />
    </MantineProvider>
  )
}

describe('DeleteAccountModal', () => {
  beforeEach(() => {
    schedule.mutate.mockClear()
    schedule.isError = false
    schedule.error = null
  })
  afterEach(cleanup)

  it('stays disabled until the email matches (case-insensitive) and the password is typed', async () => {
    renderModal(true)
    const btn = screen.getByRole('button', { name: 'Delete account' })
    expect((btn as HTMLButtonElement).disabled).toBe(true)
    await userEvent.type(screen.getByLabelText(/to confirm/i), 'me@example.com')
    expect((btn as HTMLButtonElement).disabled).toBe(true)
    await userEvent.type(screen.getByLabelText('Current password'), 'hunter22hunter')
    await userEvent.click(btn)
    expect(schedule.mutate).toHaveBeenCalledWith(
      { confirm_email: 'me@example.com', password: 'hunter22hunter' },
      expect.anything()
    )
  })

  it('sends an SSO-only account to sign in again before it can delete', () => {
    renderModal(false)
    expect(screen.queryByRole('button', { name: 'Delete account' })).toBeNull()
    const link = screen.getByRole('link', { name: 'Sign in again' })
    expect(link.getAttribute('href')).toBe('/api/auth/oidc/start?mode=reauth')
    expect(screen.getByText(/sign in again with auth\.example\.com/i)).toBeTruthy()
  })

  it('asks again when the fresh sign-in has expired', () => {
    schedule.isError = true
    schedule.error = new Error('confirm it is you: sign in again with single sign-on')
    renderModal(false, true)
    expect(screen.getByRole('link', { name: 'Sign in again' })).toBeTruthy()
    expect(screen.getByText(/more than a few minutes old/i)).toBeTruthy()
  })

  it('asks for no password on an SSO-only account that just signed in again', async () => {
    renderModal(false, true)
    expect(screen.queryByLabelText('Current password')).toBeNull()
    await userEvent.type(screen.getByLabelText(/to confirm/i), 'me@example.com')
    await userEvent.click(screen.getByRole('button', { name: 'Delete account' }))
    expect(schedule.mutate).toHaveBeenCalledWith({ confirm_email: 'me@example.com' }, expect.anything())
  })
})
