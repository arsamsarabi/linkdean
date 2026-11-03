import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { BootstrapStatus } from '@/schemas'
import { LoginPage } from './LoginPage'

type Pending = { data?: { email: string }; isError: boolean }

const state: {
  bootstrap: BootstrapStatus
  search: { link?: boolean; oidc_error?: string }
  pending: Pending
  pendingEnabled: boolean | null
  login: ReturnType<typeof vi.fn>
} = {
  search: {},
  pending: { isError: false },
  pendingEnabled: null,
  login: vi.fn(),
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
  useLogin: () => ({ isPending: false, isError: false, error: null, mutateAsync: state.login }),
  usePendingOidcLink: (enabled: boolean) => {
    state.pendingEnabled = enabled
    return state.pending
  },
}))

const notify = vi.hoisted(() => vi.fn())
vi.mock('@mantine/notifications', () => ({ notifications: { show: notify } }))
vi.mock('@/api', () => ({ api: { getSettings: () => Promise.resolve({ settings: { default_landing: '/' } }) } }))

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return { ...actual, useNavigate: () => vi.fn(), useSearch: () => state.search }
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

describe('LoginPage single sign-on link', () => {
  beforeEach(() => {
    state.search = {}
    state.pending = { isError: false }
    state.pendingEnabled = null
    state.login = vi.fn()
    notify.mockClear()
  })
  afterEach(cleanup)

  it('asks for the password of the account that holds the address', () => {
    state.search = { link: true }
    state.pending = { data: { email: 'u•••@example.com' }, isError: false }
    renderPage()
    expect(state.pendingEnabled).toBe(true)
    expect(
      screen.getByText('An account already uses u•••@example.com. Sign in with its password to connect single sign-on.')
    ).toBeTruthy()
    expect(screen.getByText("Don't know this account's password? Ask your admin.")).toBeTruthy()
  })

  it('shows nothing when the pending link is gone', () => {
    state.search = { link: true }
    state.pending = { isError: true }
    renderPage()
    expect(screen.queryByText(/already uses/)).toBeNull()
  })

  it('does not look for a pending link without link=1', () => {
    renderPage()
    expect(state.pendingEnabled).toBe(false)
    expect(screen.queryByText(/already uses/)).toBeNull()
  })

  it('says single sign-on is connected after a sign-in that linked it', async () => {
    state.login = vi.fn().mockResolvedValue({
      user: { onboarding_completed: true },
      token: 't',
      sso_connected: true,
    })
    renderPage()
    await userEvent.type(screen.getByLabelText('Email'), 'u@example.com')
    await userEvent.type(screen.getByLabelText('Password'), 'hunter22hunter')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))
    await waitFor(() => expect(notify).toHaveBeenCalledWith({ message: 'Single sign-on connected' }))
  })

  it('says nothing about single sign-on after a plain sign-in', async () => {
    state.login = vi.fn().mockResolvedValue({ user: { onboarding_completed: true }, token: 't' })
    renderPage()
    await userEvent.type(screen.getByLabelText('Email'), 'u@example.com')
    await userEvent.type(screen.getByLabelText('Password'), 'hunter22hunter')
    await userEvent.click(screen.getByRole('button', { name: 'Sign in' }))
    await waitFor(() => expect(state.login).toHaveBeenCalled())
    expect(notify).not.toHaveBeenCalled()
  })
})
