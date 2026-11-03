import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api'
import { ResetPasswordPage } from './ResetPasswordPage'

type Reset = {
  isPending: boolean
  isError: boolean
  error: Error | null
  mutateAsync: (body: unknown) => Promise<unknown>
}

const state: { reset: Reset; navigate: (to: unknown) => void } = {
  reset: { isPending: false, isError: false, error: null, mutateAsync: vi.fn() },
  navigate: vi.fn(),
}

vi.mock('@/hooks', () => ({ useResetPassword: () => state.reset }))

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return { ...actual, useNavigate: () => state.navigate }
})

function renderPage(): void {
  render(
    <MantineProvider>
      <ResetPasswordPage />
    </MantineProvider>
  )
}

const strong = 'correct horse battery'

describe('ResetPasswordPage', () => {
  beforeEach(() => {
    state.reset = { isPending: false, isError: false, error: null, mutateAsync: vi.fn().mockResolvedValue({}) }
    state.navigate = vi.fn()
    window.history.replaceState(null, '', '/reset-password#token=tok_123')
  })
  afterEach(cleanup)

  it('reads the token and removes it from the address bar', () => {
    renderPage()
    expect(window.location.hash).toBe('')
    expect(window.location.pathname).toBe('/reset-password')
    expect(screen.getByLabelText('New password')).toBeTruthy()
  })

  it('blocks a submit when the passwords do not match', async () => {
    renderPage()
    await userEvent.type(screen.getByLabelText('New password'), strong)
    await userEvent.type(screen.getByLabelText('Repeat new password'), `${strong}!`)
    await userEvent.click(screen.getByRole('button', { name: 'Set new password' }))
    expect(screen.getByText('The passwords do not match.')).toBeTruthy()
    expect(state.reset.mutateAsync).not.toHaveBeenCalled()
  })

  it('blocks a password shorter than 12 characters', async () => {
    renderPage()
    await userEvent.type(screen.getByLabelText('New password'), 'short')
    await userEvent.type(screen.getByLabelText('Repeat new password'), 'short')
    await userEvent.click(screen.getByRole('button', { name: 'Set new password' }))
    expect(screen.getByText('Use at least 12 characters.')).toBeTruthy()
    expect(state.reset.mutateAsync).not.toHaveBeenCalled()
  })

  it('shows the expired state without a token', async () => {
    window.history.replaceState(null, '', '/reset-password')
    renderPage()
    expect(screen.getByText('This link has expired or was already used.')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Request a new link' }))
    expect(state.navigate).toHaveBeenCalledWith({ to: '/forgot-password' })
  })

  it('shows the expired state when the API refuses the link', () => {
    state.reset = {
      ...state.reset,
      isError: true,
      error: new ApiError('This reset link has expired or was already used', 400, {}),
    }
    renderPage()
    expect(screen.getByText('This link has expired or was already used.')).toBeTruthy()
  })

  it('sends the token, password and checkbox, then opens the library', async () => {
    renderPage()
    await userEvent.type(screen.getByLabelText('New password'), strong)
    await userEvent.type(screen.getByLabelText('Repeat new password'), strong)
    await userEvent.click(screen.getByLabelText('Also revoke my API keys'))
    await userEvent.click(screen.getByRole('button', { name: 'Set new password' }))
    expect(state.reset.mutateAsync).toHaveBeenCalledWith({ token: 'tok_123', password: strong, revoke_api_keys: true })
    expect(state.navigate).toHaveBeenCalledWith({ to: '/library' })
  })
})
