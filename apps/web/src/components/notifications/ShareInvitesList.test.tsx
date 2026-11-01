import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ShareInvite } from '@/schemas'
import { ShareInvitesList } from './ShareInvitesList'

const accept = vi.hoisted(() => vi.fn())
const reject = vi.hoisted(() => vi.fn())
const invites = vi.hoisted(() => ({ list: [] as ShareInvite[] }))

vi.mock('@/hooks', () => ({
  useShareInvites: () => ({ data: { invites: invites.list } }),
  useAcceptShareInvite: () => ({ mutateAsync: accept, isPending: false }),
  useRejectShareInvite: () => ({ mutateAsync: reject, isPending: false }),
}))

const pending: ShareInvite = {
  id: 'i1',
  subject_type: 'category',
  subject_id: 'c1',
  subject_title: 'Reading',
  access: 'read',
  inviter_display_name: 'Ana',
  created_at: '2026-11-01T00:00:00Z',
}

describe('ShareInvitesList', () => {
  afterEach(cleanup)

  it('accepts and declines a pending invite', async () => {
    invites.list = [pending]
    render(
      <MantineProvider>
        <ShareInvitesList />
      </MantineProvider>
    )
    expect(screen.getByText(/Ana invited you to folder “Reading”/)).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Accept' }))
    expect(accept).toHaveBeenCalledWith('i1')
    await userEvent.click(screen.getByRole('button', { name: 'Decline' }))
    expect(reject).toHaveBeenCalledWith('i1')
  })

  it('leaves out declined invites and renders nothing when empty', () => {
    invites.list = [{ ...pending, rejected_at: '2026-11-01T01:00:00Z' }]
    render(
      <MantineProvider>
        <ShareInvitesList />
      </MantineProvider>
    )
    expect(screen.queryByText(/invited you/)).toBeNull()
    expect(screen.queryByRole('button', { name: 'Accept' })).toBeNull()
  })
})
