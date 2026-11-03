import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api'
import { DisconnectSsoModal } from './DisconnectSsoModal'

type Disconnect = {
  isPending: boolean
  isError: boolean
  error: Error | null
  mutate: ReturnType<typeof vi.fn>
}

const state: { disconnect: Disconnect } = {
  disconnect: { isPending: false, isError: false, error: null, mutate: vi.fn() },
}

vi.mock('@/hooks', () => ({ useDisconnectSso: () => state.disconnect }))

function renderModal(): void {
  render(
    <MantineProvider>
      <DisconnectSsoModal onClose={vi.fn()} />
    </MantineProvider>
  )
}

describe('DisconnectSsoModal', () => {
  beforeEach(() => {
    state.disconnect = { isPending: false, isError: false, error: null, mutate: vi.fn() }
  })
  afterEach(cleanup)

  it('sends the current password', async () => {
    renderModal()
    await userEvent.type(screen.getByLabelText('Current password'), 'hunter22hunter')
    await userEvent.click(screen.getByRole('button', { name: 'Disconnect' }))
    expect(state.disconnect.mutate).toHaveBeenCalledWith('hunter22hunter', expect.anything())
  })

  it('says the password is wrong on a 401', () => {
    state.disconnect = {
      ...state.disconnect,
      isError: true,
      error: new ApiError('unauthorized', 401, { error: 'unauthorized' }),
    }
    renderModal()
    expect(screen.getByText('Wrong password.')).toBeTruthy()
  })

  it('shows other refusals as the server words them', () => {
    state.disconnect = {
      ...state.disconnect,
      isError: true,
      error: new ApiError('account has no local password', 400, { error: 'account has no local password' }),
    }
    renderModal()
    expect(screen.getByText('account has no local password')).toBeTruthy()
  })
})
