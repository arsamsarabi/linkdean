import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { BannedUsersPanel } from './BannedUsersPanel'

const hooks = vi.hoisted(() => ({
  bans: [] as unknown[],
  lift: vi.fn().mockResolvedValue({}),
  change: vi.fn().mockResolvedValue({}),
}))

vi.mock('@/hooks', () => ({
  useBans: () => ({ data: { bans: hooks.bans } }),
  useLiftBans: () => ({ mutateAsync: hooks.lift, isPending: false }),
  useChangeBans: () => ({ mutateAsync: hooks.change, isPending: false }),
}))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))

const ban = (user_id: string, name: string) => ({
  user_id,
  email: `${user_id}@example.com`,
  display_name: name,
  is_admin: false,
  reason: 'Spam',
  ends_at: null,
  created_at: '2026-11-01T00:00:00Z',
})

function renderPanel() {
  return render(
    <MantineProvider>
      <BannedUsersPanel />
    </MantineProvider>
  )
}

describe('BannedUsersPanel', () => {
  afterEach(() => {
    cleanup()
    hooks.lift.mockClear()
    hooks.change.mockClear()
  })

  it('stays hidden while nobody is banned', () => {
    hooks.bans = []
    renderPanel()
    expect(screen.queryByText('Banned users')).toBeNull()
  })

  it('unbans only the selected accounts', async () => {
    hooks.bans = [ban('a', 'Ana'), ban('b', 'Ben')]
    renderPanel()
    await userEvent.click(screen.getByLabelText('Select Ben'))
    await userEvent.click(screen.getByRole('button', { name: 'Unban 1' }))
    expect(hooks.lift).toHaveBeenCalledWith(['b'])
  })

  it('changes the reason across the selection', async () => {
    hooks.bans = [ban('a', 'Ana'), ban('b', 'Ben')]
    renderPanel()
    await userEvent.click(screen.getByLabelText('Select Ana'))
    await userEvent.click(screen.getByLabelText('Select Ben'))
    await userEvent.click(screen.getByRole('button', { name: 'Change reason' }))
    await userEvent.type(await screen.findByRole('textbox', { name: /Reason/ }), 'Abuse')
    await userEvent.click(screen.getByRole('button', { name: 'Apply to 2' }))
    expect(hooks.change).toHaveBeenCalledWith({ user_ids: ['a', 'b'], reason: 'Abuse' })
  })
})
