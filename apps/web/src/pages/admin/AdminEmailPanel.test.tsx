import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { EmailStatus, TestEmailResult } from '@/schemas'
import { AdminEmailPanel } from './AdminEmailPanel'

const state: {
  status: EmailStatus | undefined
  test: { isPending: boolean; data?: TestEmailResult; error: Error | null; mutate: () => void }
} = { status: undefined, test: { isPending: false, error: null, mutate: vi.fn() } }

vi.mock('@/hooks', () => ({
  useEmailStatus: () => ({ data: state.status, isError: false, error: null, refetch: vi.fn() }),
  useSendTestEmail: () => state.test,
}))

const on: EmailStatus = {
  enabled: true,
  config: { host: 'smtp.example.com', port: 587, tls: 'starttls', from: 'Linkdean <no-reply@example.com>' },
  stats: {
    sent: 12,
    queued: 1,
    failed: 2,
    last_error: '550 mailbox unavailable',
    last_error_at: '2026-11-03T10:00:00Z',
  },
}

function renderPanel(): void {
  render(
    <MantineProvider>
      <AdminEmailPanel />
    </MantineProvider>
  )
}

describe('AdminEmailPanel', () => {
  beforeEach(() => {
    state.test = { isPending: false, error: null, mutate: vi.fn() }
  })
  afterEach(cleanup)

  it('explains how to turn mail on when it is off', () => {
    state.status = {
      enabled: false,
      config: null,
      stats: { sent: 0, queued: 0, failed: 0, last_error: null, last_error_at: null },
    }
    renderPanel()
    expect(screen.getByText('Off')).toBeTruthy()
    expect(screen.getByText('SMTP_HOST')).toBeTruthy()
    expect(screen.queryByRole('button', { name: /send test email/i })).toBeNull()
  })

  it('shows the server, counts and latest error, and sends a test', async () => {
    state.status = on
    renderPanel()
    expect(screen.getByText('On')).toBeTruthy()
    expect(screen.getByText(/smtp\.example\.com:587/)).toBeTruthy()
    expect(screen.getByText('12')).toBeTruthy()
    expect(screen.getByText(/550 mailbox unavailable/)).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: /send test email/i }))
    expect(state.test.mutate).toHaveBeenCalledTimes(1)
  })

  it('shows the SMTP error of a failed test', () => {
    state.status = on
    state.test = {
      isPending: false,
      error: null,
      mutate: vi.fn(),
      data: { ok: false, to: 'a@example.com', error: 'connection refused' },
    }
    renderPanel()
    expect(screen.getByText(/connection refused/)).toBeTruthy()
  })

  it('confirms a sent test', () => {
    state.status = on
    state.test = { isPending: false, error: null, mutate: vi.fn(), data: { ok: true, to: 'a@example.com' } }
    renderPanel()
    expect(screen.getByText(/test email sent to a@example\.com/i)).toBeTruthy()
  })
})
