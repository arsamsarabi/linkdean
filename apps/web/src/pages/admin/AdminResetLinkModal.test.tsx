import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { AdminResetLink } from '@/schemas'
import { AdminResetLinkModal } from './AdminResetLinkModal'

type Create = {
  isPending: boolean
  isError: boolean
  error: Error | null
  data?: AdminResetLink
  mutate: (id: string) => void
}

const state: { create: Create; mail: boolean } = {
  create: { isPending: false, isError: false, error: null, mutate: vi.fn() },
  mail: false,
}

vi.mock('@/hooks', () => ({
  useCreateResetLink: () => state.create,
  useBootstrap: () => ({ data: { mail_enabled: state.mail } }),
}))

function renderModal(): ReturnType<typeof render> {
  return render(
    <MantineProvider>
      <AdminResetLinkModal user={{ id: 'u1', label: 'Ada' }} onClose={vi.fn()} />
    </MantineProvider>
  )
}

const url = 'https://links.example.com/reset-password#token=abc'

describe('AdminResetLinkModal', () => {
  beforeEach(() => {
    state.create = { isPending: false, isError: false, error: null, mutate: vi.fn() }
    state.mail = false
  })
  afterEach(cleanup)

  it('creates a link only when asked, then shows it with a copy button', async () => {
    state.create.mutate = vi.fn(() => {
      state.create = { ...state.create, data: { url, expires_in_hours: 24 } }
    })
    const mutate = state.create.mutate
    const view = renderModal()
    expect(mutate).not.toHaveBeenCalled()
    expect(screen.queryByDisplayValue(url)).toBeNull()
    await userEvent.click(screen.getByRole('button', { name: 'Create link' }))
    expect(mutate).toHaveBeenCalledWith('u1')
    view.rerender(
      <MantineProvider>
        <AdminResetLinkModal user={{ id: 'u1', label: 'Ada' }} onClose={vi.fn()} />
      </MantineProvider>
    )
    expect(screen.getByDisplayValue(url)).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Copy reset link' })).toBeTruthy()
    expect(screen.getByText(/Send it privately\. Anyone with this link can set the password\./)).toBeTruthy()
    expect(screen.queryByText('We also emailed them that a link was created.')).toBeNull()
  })

  it('mentions the notice email when mail is on', () => {
    state.mail = true
    state.create = { ...state.create, data: { url, expires_in_hours: 24 } }
    renderModal()
    expect(screen.getByText('We also emailed them that a link was created.')).toBeTruthy()
  })
})
