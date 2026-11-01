import type { ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Notification, ShareInvite } from '@/schemas'
import { NotificationsDrawer } from './NotificationsDrawer'

const markAll = vi.hoisted(() => vi.fn())
const data = vi.hoisted(() => ({ rows: [] as Notification[], invites: [] as ShareInvite[] }))

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => vi.fn() }))
vi.mock('./ShareInvitesList', () => ({ ShareInvitesList: () => null }))

vi.mock('@/hooks', () => ({
  useShareInvites: () => ({ data: { invites: data.invites } }),
  useNotifications: () => ({ data: { notifications: data.rows }, isLoading: false, isError: false }),
  useUnreadNotificationCount: () => ({ data: { count: 2 } }),
  useMarkNotificationRead: () => ({ mutateAsync: vi.fn() }),
  useMarkAllNotificationsRead: () => ({ mutateAsync: markAll, isPending: false }),
}))

function drawer(opened: boolean): ReactElement {
  return (
    <MantineProvider>
      <NotificationsDrawer opened={opened} onClose={vi.fn()} />
    </MantineProvider>
  )
}

const inviteRow: Notification = {
  id: 'n1',
  kind: 'share_invite',
  subject_type: 'category',
  subject_id: 'c1',
  subject_title: 'Reading',
  body: 'Ana invited you to folder “Reading” (read-only)',
  payload: {},
  created_at: '2026-11-01T00:00:00Z',
}

const invite: ShareInvite = {
  id: 'i1',
  subject_type: 'category',
  subject_id: 'c1',
  subject_title: 'Reading',
  access: 'read',
  inviter_display_name: 'Ana',
  created_at: '2026-11-01T00:00:00Z',
}

describe('NotificationsDrawer', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    markAll.mockResolvedValue({ ok: true })
    data.rows = []
    data.invites = []
  })
  afterEach(cleanup)

  it('keeps notifications unread when opening', () => {
    const view = render(drawer(false))
    view.rerender(drawer(true))
    expect(markAll).not.toHaveBeenCalled()
  })

  it('marks everything read when closing', () => {
    const view = render(drawer(true))
    view.rerender(drawer(false))
    expect(markAll).toHaveBeenCalledTimes(1)
  })

  it('shows an invite row only once the invite has been answered', () => {
    data.rows = [inviteRow]
    data.invites = [invite]
    const view = render(drawer(true))
    expect(screen.queryByText(inviteRow.body)).toBeNull()
    expect(screen.queryByText('No notifications yet.')).toBeNull()
    view.unmount()

    data.invites = []
    render(drawer(true))
    expect(screen.getByText(inviteRow.body)).toBeTruthy()
  })
})
