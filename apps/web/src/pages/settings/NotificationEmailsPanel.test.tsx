import { type ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
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
  { kind: 'share_invite', label: 'Share invites', enabled: true, default: true },
  { kind: 'social_comment', label: 'Comments', enabled: false, default: false },
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

function toggle(name: string): HTMLInputElement {
  return screen.getByRole<HTMLInputElement>('switch', { name: new RegExp(`^${name}`) })
}

describe('NotificationEmailsPanel', () => {
  beforeEach(() => {
    api.bootstrap.mockResolvedValue({ mail_enabled: true })
    api.notificationEmailPrefs.mockResolvedValue({ prefs: PREFS })
    api.setNotificationEmailPref.mockReset()
  })
  afterEach(cleanup)

  it('shows one switch per kind, with the default hint', async () => {
    render(ui())
    expect((await screen.findByRole('switch', { name: /^Share invites/ })) as HTMLInputElement).toBeTruthy()
    expect(toggle('Share invites').checked).toBe(true)
    expect(toggle('Comments').checked).toBe(false)
    expect(screen.getAllByText('On by default')).toHaveLength(1)
    expect(screen.queryByText(/need outbound email/)).toBeNull()
  })

  it('saves on toggle and shows the new value at once', async () => {
    let resolve: (p: NotificationEmailPref) => void = () => {}
    api.setNotificationEmailPref.mockReturnValue(new Promise((r) => (resolve = r)))
    render(ui())
    await screen.findByRole('switch', { name: /^Comments/ })
    await waitFor(() => expect(toggle('Comments').disabled).toBe(false))
    await userEvent.click(toggle('Comments'))
    expect(api.setNotificationEmailPref).toHaveBeenCalledWith('social_comment', true)
    await waitFor(() => expect(toggle('Comments').checked).toBe(true))
    resolve({ kind: 'social_comment', label: 'Comments', enabled: true, default: false })
    await waitFor(() => expect(toggle('Comments').checked).toBe(true))
  })

  it('puts the switch back when saving fails', async () => {
    let reject: (e: Error) => void = () => {}
    api.setNotificationEmailPref.mockReturnValue(new Promise((_, r) => (reject = r)))
    render(ui())
    await screen.findByRole('switch', { name: /^Share invites/ })
    await waitFor(() => expect(toggle('Share invites').disabled).toBe(false))
    await userEvent.click(toggle('Share invites'))
    expect(api.setNotificationEmailPref).toHaveBeenCalledWith('share_invite', false)
    await waitFor(() => expect(toggle('Share invites').checked).toBe(false))
    reject(new Error('nope'))
    await waitFor(() => expect(toggle('Share invites').checked).toBe(true))
  })

  it('explains and disables the switches with mail off', async () => {
    api.bootstrap.mockResolvedValue({ mail_enabled: false })
    render(ui())
    expect(await screen.findByText('Email notifications need outbound email. Ask your admin.')).toBeTruthy()
    expect(toggle('Share invites').disabled).toBe(true)
    expect(toggle('Comments').disabled).toBe(true)
  })
})
