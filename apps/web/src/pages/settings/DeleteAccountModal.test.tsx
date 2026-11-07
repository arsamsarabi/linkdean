import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { DeleteAccountModal } from './DeleteAccountModal'

const schedule = { isPending: false, isError: false, error: null, mutate: vi.fn() }

vi.mock('@/hooks', () => ({ useScheduleAccountDeletion: () => schedule }))

function renderModal(hasPassword: boolean): void {
  render(
    <MantineProvider>
      <DeleteAccountModal email="Me@Example.com" hasPassword={hasPassword} onClose={vi.fn()} />
    </MantineProvider>
  )
}

describe('DeleteAccountModal', () => {
  beforeEach(() => schedule.mutate.mockClear())
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

  it('asks for no password on an SSO-only account', async () => {
    renderModal(false)
    expect(screen.queryByLabelText('Current password')).toBeNull()
    await userEvent.type(screen.getByLabelText(/to confirm/i), 'me@example.com')
    await userEvent.click(screen.getByRole('button', { name: 'Delete account' }))
    expect(schedule.mutate).toHaveBeenCalledWith({ confirm_email: 'me@example.com' }, expect.anything())
  })
})
