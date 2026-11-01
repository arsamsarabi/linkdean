import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api/client'
import { MoveToVaultModal } from './MoveToVaultModal'

// jsdom has no ResizeObserver or scrollIntoView; Mantine's Select needs both.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
)
Element.prototype.scrollIntoView = () => undefined

const move = vi.hoisted(() => vi.fn())
const hooks = vi.hoisted(() => ({ rejectWith: null as Error | null }))

vi.mock('@tanstack/react-router', () => ({
  Link: ({ children, to }: { children: unknown; to: string }) => <a href={to}>{children as string}</a>,
}))

vi.mock('@/hooks', () => ({
  useVaults: () => ({
    data: {
      vaults: [
        { id: 'v1', name: 'Work', unlocked: true, has_recovery_key: true, in_global: false },
        { id: 'v2', name: 'Home', unlocked: false, has_recovery_key: true, in_global: false },
      ],
    },
  }),
  useMoveToVault: () => ({
    // A plain function: rejecting through a vi.fn failed this test even though the modal caught it.
    mutateAsync: async (body: unknown) => {
      move(body)
      if (hooks.rejectWith) throw hooks.rejectWith
    },
    isPending: false,
  }),
}))

function modal(onClose = vi.fn()) {
  return (
    <MantineProvider>
      <MoveToVaultModal categoryIds={[]} bookmarkIds={['b1']} onClose={onClose} />
    </MantineProvider>
  )
}

async function openDestination(): Promise<void> {
  const input = screen.getAllByLabelText('Destination').find((el) => el.tagName === 'INPUT')
  await userEvent.click(input as HTMLElement)
}

describe('MoveToVaultModal', () => {
  beforeEach(() => {
    move.mockReset()
    hooks.rejectWith = null
  })
  afterEach(cleanup)

  it('offers only unlocked vaults and moves on confirm', async () => {
    move.mockResolvedValue({ ok: true })
    const onClose = vi.fn()
    render(modal(onClose))
    await openDestination()
    expect(screen.queryByRole('option', { name: 'Home', hidden: true })).toBeNull()
    await userEvent.click(await screen.findByRole('option', { name: 'Work', hidden: true }))
    await userEvent.click(screen.getByRole('button', { name: 'Move' }))
    expect(move).toHaveBeenCalledWith({ category_ids: [], bookmark_ids: ['b1'], destination_vault_id: 'v1' })
    expect(onClose).toHaveBeenCalled()
  })

  it('lists every blocker and stays open', async () => {
    const blocked = new ApiError('This move is blocked.', 409, {
      blockers: [
        { kind: 'shared', subject_id: 'c1', message: 'Remove 2 people from sharing on “Reading”' },
        { kind: 'dashboard', subject_id: 'b1', related_id: 'd1', message: 'Remove “Rust” from dashboard “Home”' },
      ],
    })
    hooks.rejectWith = blocked
    const onClose = vi.fn()
    render(modal(onClose))
    await openDestination()
    await userEvent.click(await screen.findByRole('option', { name: 'Work', hidden: true }))
    await userEvent.click(screen.getByRole('button', { name: 'Move' }))
    expect(await screen.findByText('Remove 2 people from sharing on “Reading”')).toBeTruthy()
    expect(screen.getByText('Remove “Rust” from dashboard “Home”')).toBeTruthy()
    expect(onClose).not.toHaveBeenCalled()
  })
})
