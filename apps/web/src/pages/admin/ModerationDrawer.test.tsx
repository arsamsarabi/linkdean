import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ModerationDrawer } from './ModerationDrawer'

const moderate = vi.hoisted(() => vi.fn().mockResolvedValue({ outcome: { applied: 1, skipped: 0 } }))

const content = {
  public_links: [{ id: 'c1', name: 'Links', visibility: 'private', public_slug: 'ldl_1' }],
  categories: [],
  bookmarks: [{ id: 'b1', title: 'Rude site', url: 'https://rude.example/' }],
  profile_visibility: null,
  shares: [],
  comments: [],
}

vi.mock('@/hooks', () => ({
  useExposedContent: () => ({ data: { content } }),
  useModerationActions: () => ({
    data: {
      actions: [
        {
          id: 'm1',
          action: 'comment_deleted',
          subject_title: 'rude words',
          reason: 'Abuse',
          admin_email: 'admin@example.com',
          created_at: '2026-11-01T00:00:00Z',
        },
      ],
    },
  }),
  useModerate: () => ({ mutateAsync: moderate, isPending: false }),
}))
vi.mock('@/components', () => ({
  ConfirmModal: ({ opened, onConfirm, body }: { opened: boolean; onConfirm: () => void; body: string }) =>
    opened ? <button onClick={onConfirm}>Confirm: {body}</button> : null,
  QueryErrorAlert: () => null,
}))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))

function renderDrawer() {
  render(
    <MantineProvider env="test">
      <ModerationDrawer user={{ id: 'u1', label: 'Ana' }} onClose={() => undefined} />
    </MantineProvider>
  )
}

async function fillNote(): Promise<void> {
  await userEvent.type(await screen.findByRole('textbox', { name: /Note/ }), 'Policy')
}

const buttonNamed = (name: string) => screen.findByRole('button', { name })

describe('ModerationDrawer', () => {
  afterEach(() => {
    cleanup()
    moderate.mockClear()
  })

  it('needs a selection and a note before acting', async () => {
    renderDrawer()
    const turnOff = (await buttonNamed('Turn off')) as HTMLButtonElement
    expect(turnOff.disabled).toBe(true)
    await userEvent.click(screen.getByRole('checkbox', { name: /Links/ }))
    expect(turnOff.disabled).toBe(true)
    await fillNote()
    expect(turnOff.disabled).toBe(false)
  })

  it('applies a plain action straight away', async () => {
    renderDrawer()
    await fillNote()
    await userEvent.click(screen.getByRole('checkbox', { name: /Links/ }))
    await userEvent.click(await buttonNamed('Turn off'))
    expect(moderate).toHaveBeenCalledWith({
      reason: 'Policy',
      actions: [{ action: 'public_link_disabled', subject_type: undefined, subject_id: 'c1' }],
    })
  })

  it('asks before moving to Trash', async () => {
    renderDrawer()
    await fillNote()
    await userEvent.click(screen.getByRole('checkbox', { name: /Rude site/ }))
    await userEvent.click(await buttonNamed('Move to Trash'))
    expect(moderate).not.toHaveBeenCalled()
    await userEvent.click(await screen.findByRole('button', { name: /^Confirm/ }))
    expect(moderate).toHaveBeenCalledWith({
      reason: 'Policy',
      actions: [{ action: 'removed_to_trash', subject_type: 'bookmark', subject_id: 'b1' }],
    })
  })

  it('lists the history', async () => {
    renderDrawer()
    expect(await screen.findByText('comment deleted: rude words')).toBeTruthy()
  })
})
