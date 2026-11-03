import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { VerifyEmailBanner } from './VerifyEmailBanner'

type Resend = { isPending: boolean; isSuccess: boolean; isError: boolean; error: Error | null; mutate: () => void }

const state: { resend: Resend } = {
  resend: { isPending: false, isSuccess: false, isError: false, error: null, mutate: vi.fn() },
}

vi.mock('@/hooks', () => ({ useResendVerification: () => state.resend }))

function renderBanner(props: { unverified: boolean; mailEnabled: boolean }): ReturnType<typeof render> {
  return render(
    <MantineProvider>
      <VerifyEmailBanner {...props} />
    </MantineProvider>
  )
}

const text = 'Verify your email so people can share with you.'

describe('VerifyEmailBanner', () => {
  beforeEach(() => {
    state.resend = { isPending: false, isSuccess: false, isError: false, error: null, mutate: vi.fn() }
    sessionStorage.clear()
  })
  afterEach(cleanup)

  it('shows for an unverified account when mail is on', () => {
    renderBanner({ unverified: true, mailEnabled: true })
    expect(screen.getByText(text)).toBeTruthy()
  })

  it('hides when mail is off, since nobody can verify', () => {
    renderBanner({ unverified: true, mailEnabled: false })
    expect(screen.queryByText(text)).toBeNull()
  })

  it('hides for a verified account', () => {
    renderBanner({ unverified: false, mailEnabled: true })
    expect(screen.queryByText(text)).toBeNull()
  })

  it('resends and then says Sent', async () => {
    const view = renderBanner({ unverified: true, mailEnabled: true })
    await userEvent.click(screen.getByRole('button', { name: 'Resend email' }))
    expect(state.resend.mutate).toHaveBeenCalledTimes(1)
    state.resend = { ...state.resend, isSuccess: true }
    view.rerender(
      <MantineProvider>
        <VerifyEmailBanner unverified mailEnabled />
      </MantineProvider>
    )
    expect(screen.getByRole('button', { name: 'Sent' })).toBeTruthy()
  })

  it('shows the API message when rate limited', () => {
    state.resend = { ...state.resend, isError: true, error: new Error('too many attempts, try again later') }
    renderBanner({ unverified: true, mailEnabled: true })
    expect(screen.getByText('too many attempts, try again later')).toBeTruthy()
  })

  it('stays dismissed for the session', async () => {
    renderBanner({ unverified: true, mailEnabled: true })
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }))
    expect(screen.queryByText(text)).toBeNull()
    cleanup()
    renderBanner({ unverified: true, mailEnabled: true })
    expect(screen.queryByText(text)).toBeNull()
  })
})
