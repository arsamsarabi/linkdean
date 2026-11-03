import { StrictMode, type ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api'
import { VerifyEmailPage } from './VerifyEmailPage'

const api = vi.hoisted(() => ({
  verifyEmail: vi.fn(),
  confirmEmailChange: vi.fn(),
  me: vi.fn(),
  resendVerification: vi.fn(),
  logout: vi.fn(),
}))
const navigate = vi.hoisted(() => vi.fn())

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return { ...actual, api }
})
vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return { ...actual, useNavigate: () => navigate }
})

function ui(): ReactElement {
  return (
    <StrictMode>
      <MantineProvider>
        <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
          <VerifyEmailPage />
        </QueryClientProvider>
      </MantineProvider>
    </StrictMode>
  )
}

const signedOut = new ApiError('unauthorized', 401, {})
const spent = new ApiError('this verification link has expired or was already used', 400, {})

function meResponse(over: { email_verified?: boolean; is_admin?: boolean; require?: boolean } = {}) {
  return {
    user: { email: 'ada@example.com', email_verified: over.email_verified ?? false, is_admin: over.is_admin ?? false },
    require_verified_email: over.require ?? true,
  }
}

let tokenSeq = 0

describe('VerifyEmailPage with a link', () => {
  beforeEach(() => {
    tokenSeq += 1
    for (const fn of Object.values(api)) fn.mockReset()
    navigate.mockReset()
    api.me.mockRejectedValue(signedOut)
  })
  afterEach(cleanup)

  it('verifies once, clears the fragment and offers sign-in', async () => {
    window.history.replaceState(null, '', `/verify-email#token=tok_${tokenSeq}`)
    api.verifyEmail.mockResolvedValue({ ok: true })
    render(ui())
    expect(await screen.findByText('Email verified')).toBeTruthy()
    expect(window.location.hash).toBe('')
    expect(api.verifyEmail).toHaveBeenCalledTimes(1)
    expect(api.verifyEmail).toHaveBeenCalledWith(`tok_${tokenSeq}`)
    expect(await screen.findByRole('button', { name: 'Sign in' })).toBeTruthy()
  })

  it('confirms an email change and continues to the library when signed in', async () => {
    window.history.replaceState(null, '', `/verify-email#token=tok_${tokenSeq}&change=1`)
    api.confirmEmailChange.mockResolvedValue({ ok: true })
    api.me.mockResolvedValue(meResponse({ email_verified: true }))
    render(ui())
    expect(await screen.findByText('Email changed')).toBeTruthy()
    expect(api.confirmEmailChange).toHaveBeenCalledTimes(1)
    expect(api.verifyEmail).not.toHaveBeenCalled()
    await userEvent.click(await screen.findByRole('button', { name: 'Continue to your library' }))
    expect(navigate).toHaveBeenCalledWith({ to: '/library' })
  })

  it('says a spent link has expired and offers a new one when signed in', async () => {
    window.history.replaceState(null, '', `/verify-email#token=tok_${tokenSeq}`)
    api.verifyEmail.mockRejectedValue(spent)
    api.me.mockResolvedValue(meResponse())
    api.resendVerification.mockResolvedValue({ ok: true, verified: false, hours: 24 })
    render(ui())
    expect(await screen.findByText('This link has expired or was already used.')).toBeTruthy()
    await userEvent.click(await screen.findByRole('button', { name: 'Send a new link' }))
    expect(await screen.findByText('We sent a new link. It works for 24 hours.')).toBeTruthy()
    expect(api.resendVerification).toHaveBeenCalledTimes(1)
  })

  it('offers sign-in, not a new link, when signed out', async () => {
    window.history.replaceState(null, '', `/verify-email#token=tok_${tokenSeq}`)
    api.verifyEmail.mockRejectedValue(spent)
    render(ui())
    expect(await screen.findByText('This link has expired or was already used.')).toBeTruthy()
    expect(await screen.findByRole('button', { name: 'Sign in' })).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Send a new link' })).toBeNull()
  })
})

describe('VerifyEmailPage as the hard gate', () => {
  beforeEach(() => {
    for (const fn of Object.values(api)) fn.mockReset()
    navigate.mockReset()
    window.history.replaceState(null, '', '/verify-email')
  })
  afterEach(cleanup)

  it('shows the address with Resend and Sign out', async () => {
    api.me.mockResolvedValue(meResponse())
    api.resendVerification.mockResolvedValue({ ok: true, verified: false, hours: 24 })
    render(ui())
    expect(await screen.findByText('Verify your email')).toBeTruthy()
    expect(screen.getByText('ada@example.com')).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Sign out' })).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Resend email' }))
    expect(await screen.findByRole('button', { name: 'Sent' })).toBeTruthy()
  })

  it('shows the API message when resend is rate limited', async () => {
    api.me.mockResolvedValue(meResponse())
    api.resendVerification.mockRejectedValue(new ApiError('too many attempts, try again later', 429, {}))
    render(ui())
    await userEvent.click(await screen.findByRole('button', { name: 'Resend email' }))
    expect(await screen.findByText('too many attempts, try again later')).toBeTruthy()
  })

  it('returns to the app once the account may pass', async () => {
    api.me.mockResolvedValue(meResponse({ email_verified: true }))
    render(ui())
    await waitFor(() => expect(navigate).toHaveBeenCalledWith({ to: '/', replace: true }))
  })

  it('lets admins through', async () => {
    api.me.mockResolvedValue(meResponse({ is_admin: true }))
    render(ui())
    await waitFor(() => expect(navigate).toHaveBeenCalledWith({ to: '/', replace: true }))
  })

  it('goes to sign in when there is no session', async () => {
    api.me.mockRejectedValue(signedOut)
    render(ui())
    await waitFor(() => expect(navigate).toHaveBeenCalledWith({ to: '/login', replace: true }))
  })
})
