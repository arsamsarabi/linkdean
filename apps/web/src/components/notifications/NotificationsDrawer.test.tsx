import type { ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { NotificationsDrawer } from './NotificationsDrawer'

const markAll = vi.hoisted(() => vi.fn())

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => vi.fn() }))

vi.mock('@/hooks', () => ({
  useNotifications: () => ({ data: { notifications: [] }, isLoading: false, isError: false }),
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

describe('NotificationsDrawer', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    markAll.mockResolvedValue({ ok: true })
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
})
