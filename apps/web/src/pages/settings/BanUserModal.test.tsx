import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { BanUserModal } from './BanUserModal'

const ban = vi.hoisted(() => vi.fn().mockResolvedValue({}))

vi.mock('@/hooks', () => ({ useBanUser: () => ({ mutateAsync: ban, isPending: false }) }))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))

function renderModal() {
  const onClose = vi.fn()
  render(
    <MantineProvider>
      <BanUserModal user={{ id: 'u1', label: 'Ana' }} onClose={onClose} />
    </MantineProvider>
  )
  return onClose
}

const submit = () => screen.findByRole('button', { name: 'Ban' })

describe('BanUserModal', () => {
  afterEach(() => {
    cleanup()
    ban.mockClear()
  })

  it('needs a reason', async () => {
    renderModal()
    expect(((await submit()) as HTMLButtonElement).disabled).toBe(true)
  })

  it('leaves out a duration for a permanent ban', async () => {
    const onClose = renderModal()
    await userEvent.type(await screen.findByRole('textbox', { name: /Reason/ }), 'Spam')
    await userEvent.click(await submit())
    expect(ban).toHaveBeenCalledWith({ id: 'u1', reason: 'Spam', days: undefined, ends_at: undefined })
    expect(onClose).toHaveBeenCalled()
  })

  it('sends a number of days', async () => {
    renderModal()
    await userEvent.type(await screen.findByRole('textbox', { name: /Reason/ }), 'Spam')
    await userEvent.click(screen.getByRole('radio', { name: 'Number of days' }))
    await userEvent.click(await submit())
    expect(ban).toHaveBeenCalledWith({ id: 'u1', reason: 'Spam', days: 7, ends_at: undefined })
  })
})
