import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { BootstrapStatus } from '@/schemas'
import { LoginPage } from './LoginPage'

const state: { bootstrap: BootstrapStatus } = {
  bootstrap: {
    needs_bootstrap: false,
    bootstrap_token_required: false,
    oidc_enabled: false,
    registration_open: true,
    mail_enabled: false,
  },
}

vi.mock('@/hooks', () => ({
  useBootstrap: () => ({ data: state.bootstrap, isLoading: false, isError: false, error: null, refetch: vi.fn() }),
  useLogin: () => ({ isPending: false, isError: false, error: null, mutateAsync: vi.fn() }),
}))

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return { ...actual, useNavigate: () => vi.fn(), useSearch: () => ({}) }
})

function renderPage(): void {
  render(
    <MantineProvider>
      <LoginPage />
    </MantineProvider>
  )
}

describe('LoginPage forgotten password', () => {
  afterEach(cleanup)

  it('links to the forgot password page when mail is on', () => {
    state.bootstrap = { ...state.bootstrap, mail_enabled: true }
    renderPage()
    expect(screen.getByRole('button', { name: 'Forgot password?' })).toBeTruthy()
    expect(screen.queryByText(/ask your admin/i)).toBeNull()
  })

  it('points to the admin when mail is off', () => {
    state.bootstrap = { ...state.bootstrap, mail_enabled: false }
    renderPage()
    expect(screen.getByText('Forgot your password? Ask your admin for a reset link.')).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Forgot password?' })).toBeNull()
  })
})
