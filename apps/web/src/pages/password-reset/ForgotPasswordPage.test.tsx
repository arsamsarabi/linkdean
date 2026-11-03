import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ForgotPasswordResponse } from '@/schemas'
import { ForgotPasswordPage } from './ForgotPasswordPage'

type Forgot = {
  isPending: boolean
  isError: boolean
  error: Error | null
  data?: ForgotPasswordResponse
  mutate: (email: string) => void
}

const state: { forgot: Forgot } = { forgot: { isPending: false, isError: false, error: null, mutate: vi.fn() } }

vi.mock('@/hooks', () => ({ useForgotPassword: () => state.forgot }))

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return { ...actual, useNavigate: () => vi.fn() }
})

function renderPage(): ReturnType<typeof render> {
  return render(
    <MantineProvider>
      <ForgotPasswordPage />
    </MantineProvider>
  )
}

const confirmation = "If an account exists for that address, we've sent a link. It works for 30 minutes."

describe('ForgotPasswordPage', () => {
  beforeEach(() => {
    state.forgot = { isPending: false, isError: false, error: null, mutate: vi.fn() }
  })
  afterEach(cleanup)

  it('sends the address and shows the same confirmation for any address', async () => {
    for (const email of ['known@example.com', 'nobody@example.com']) {
      state.forgot = {
        isPending: false,
        isError: false,
        error: null,
        mutate: vi.fn(() => {
          state.forgot = { ...state.forgot, data: { ok: true, minutes: 30 } }
        }),
      }
      const view = renderPage()
      await userEvent.type(screen.getByLabelText('Email'), email)
      const mutate = state.forgot.mutate
      await userEvent.click(screen.getByRole('button', { name: 'Send reset link' }))
      expect(mutate).toHaveBeenCalledWith(email)
      view.rerender(
        <MantineProvider>
          <ForgotPasswordPage />
        </MantineProvider>
      )
      expect(screen.getByText(confirmation)).toBeTruthy()
      expect(screen.queryByLabelText('Email')).toBeNull()
      expect(screen.getByRole('button', { name: /back to sign in/i })).toBeTruthy()
      cleanup()
    }
  })

  it('shows the rate limit message', () => {
    state.forgot = { ...state.forgot, isError: true, error: new Error('Too many requests, try again later') }
    renderPage()
    expect(screen.getByText('Too many requests, try again later')).toBeTruthy()
  })
})
