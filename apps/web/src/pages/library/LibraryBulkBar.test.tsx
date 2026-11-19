import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { LibraryBulkBar } from './LibraryBulkBar'

const lib = vi.hoisted(() => ({
  selectMode: true,
  selectedCount: 3,
  allSelected: false,
  selectingAll: false,
  matchFilter: null,
  total: 3,
  selectableIds: ['mine', 'theirs', 'far'],
  selectedIds: new Set(['mine', 'theirs', 'far']),
  selectedItems: [
    { id: 'mine', owner_user_id: 'me' },
    { id: 'theirs', owner_user_id: 'friend' },
  ],
  toggleSelectAll: vi.fn(),
  setBulkMode: vi.fn(),
  setMoveTarget: vi.fn(),
  setBulkDeleteOpen: vi.fn(),
}))

vi.mock('./useLibraryPageModel', () => ({ useLibraryPageModel: () => lib }))
vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useMe: () => ({ data: { user: { id: 'me' } } }),
  useArchivesEnabled: () => true,
}))

describe('LibraryBulkBar', () => {
  afterEach(cleanup)

  it('moves only your own bookmarks to a vault', async () => {
    render(
      <MantineProvider>
        <LibraryBulkBar />
      </MantineProvider>
    )
    await userEvent.click(screen.getByRole('button', { name: 'Move 2 to vault' }))
    expect(lib.setMoveTarget).toHaveBeenCalledWith({ categoryIds: [], bookmarkIds: ['mine', 'far'] })
  })
})
