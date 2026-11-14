import { type ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import type { NotificationEmailPref } from '@/schemas'
import { NotificationEmailsPanel } from './NotificationEmailsPanel'

const api = vi.hoisted(() => ({
  bootstrap: vi.fn(),
  notificationEmailPrefs: vi.fn(),
  setNotificationEmailPref: vi.fn(),
}))

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return { ...actual, api }
})

const PREFS: NotificationEmailPref[] = [
  {
    kind: 'share_invite',
    label: 'Share invites',
    enabled: true,
    default: true,
    delivery: 'instant',
    digest_allowed: false,
  },
  {
    kind: 'social_comment',
    label: 'Comments',
    enabled: false,
    default: false,
    delivery: 'instant',
    digest_allowed: true,
  },
]

function ui(): ReactElement {
  return (
    <MantineProvider>
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <NotificationEmailsPanel />
      </QueryClientProvider>
    </MantineProvider>
  )
}

function choice(kind: string, label: string): HTMLInputElement {
  const group = screen.getByRole('radiogroup', { name: `Email for ${kind}` })
  return within(group).getByRole<HTMLInputElement>('radio', { name: label })
}

describe('NotificationEmailsPanel', () => {
  beforeAll(() => {
    // SegmentedControl measures its options.
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {}
        unobserve(): void {}
        disconnect(): void {}
      }
    )
  })
  beforeEach(() => {
    api.bootstrap.mockResolvedValue({ mail_enabled: true })
    api.notificationEmailPrefs.mockResolvedValue({ prefs: PREFS })
    api.setNotificationEmailPref.mockReset()
  })
  afterEach(cleanup)

  it('shows each kind with its current choice, and the digest only where it is allowed', async () => {
    render(ui())
    await screen.findByRole('radiogroup', { name: 'Email for Share invites' })
    expect(choice('Share invites', 'Each one').checked).toBe(true)
    expect(choice('Comments', 'Off').checked).toBe(true)
    expect(choice('Comments', 'Daily digest')).toBeTruthy()
    const invites = screen.getByRole('radiogroup', { name: 'Email for Share invites' })
    expect(within(invites).queryByRole('radio', { name: 'Daily digest' })).toBeNull()
    expect(screen.getAllByText('On by default')).toHaveLength(1)
    expect(screen.queryByText(/need outbound email/)).toBeNull()
  })

  it('saves the daily digest and shows it at once', async () => {
    let resolve: (p: NotificationEmailPref) => void = () => {}
    api.setNotificationEmailPref.mockReturnValue(new Promise((r) => (resolve = r)))
    render(ui())
    await screen.findByRole('radiogroup', { name: 'Email for Comments' })
    await waitFor(() => expect(choice('Comments', 'Daily digest').disabled).toBe(false))
    await userEvent.click(choice('Comments', 'Daily digest'))
    expect(api.setNotificationEmailPref).toHaveBeenCalledWith('social_comment', true, 'daily')
    await waitFor(() => expect(choice('Comments', 'Daily digest').checked).toBe(true))
    resolve({ ...PREFS[1], enabled: true, delivery: 'daily' })
    await waitFor(() => expect(choice('Comments', 'Daily digest').checked).toBe(true))
  })

  it('turns a kind off and puts it back when saving fails', async () => {
    let reject: (e: Error) => void = () => {}
    api.setNotificationEmailPref.mockReturnValue(new Promise((_, r) => (reject = r)))
    render(ui())
    await screen.findByRole('radiogroup', { name: 'Email for Share invites' })
    await waitFor(() => expect(choice('Share invites', 'Off').disabled).toBe(false))
    await userEvent.click(choice('Share invites', 'Off'))
    expect(api.setNotificationEmailPref).toHaveBeenCalledWith('share_invite', false, undefined)
    await waitFor(() => expect(choice('Share invites', 'Off').checked).toBe(true))
    reject(new Error('nope'))
    await waitFor(() => expect(choice('Share invites', 'Each one').checked).toBe(true))
  })

  it('explains and disables the choices with mail off', async () => {
    api.bootstrap.mockResolvedValue({ mail_enabled: false })
    render(ui())
    expect(await screen.findByText('Email notifications need outbound email. Ask your admin.')).toBeTruthy()
    expect(choice('Share invites', 'Off').disabled).toBe(true)
    expect(choice('Comments', 'Daily digest').disabled).toBe(true)
  })
})
