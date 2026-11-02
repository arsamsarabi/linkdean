import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { LibraryGridView } from './LibraryGridView'

const lib = vi.hoisted(() => ({
  loading: false,
  emptyLibrary: false,
  emptyFiltered: true,
  folderCat: { id: 'f1', name: 'Typography' } as { id: string; name: string } | null,
  searching: false,
  filterActive: false,
  debouncedQ: '',
  canCreateInContext: true,
  scope: 'library',
  viewMode: 'grid',
  list: [],
  openCreate: vi.fn(),
  setQ: vi.fn(),
  setLibrarySearch: vi.fn(),
  selectedIds: new Set<string>(),
  me: { data: undefined },
}))

vi.mock('./useLibraryPageModel', () => ({ useLibraryPageModel: () => lib }))
vi.mock('./LibraryFilterPanel', () => ({ LibraryFilterPanel: () => null }))
vi.mock('@/components', async () => {
  const { EmptyState } = await import('@/components/empty-state')
  return { EmptyState, BookmarkCard: () => null, BookmarkGridSkeleton: () => null }
})

function renderView(): void {
  render(
    <MantineProvider env="test">
      <LibraryGridView />
    </MantineProvider>
  )
}

describe('LibraryGridView', () => {
  afterEach(() => {
    cleanup()
    Object.assign(lib, { searching: false, filterActive: false })
  })

  it('offers to add into an empty folder instead of saying nothing matched', async () => {
    renderView()
    expect(screen.getByText('This folder is empty')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Add bookmark here' }))
    expect(lib.openCreate).toHaveBeenCalled()
  })

  it('still says nothing matched when a search finds nothing in the folder', () => {
    Object.assign(lib, { searching: true, debouncedQ: 'zzz' })
    renderView()
    expect(screen.getByText('Nothing matched')).toBeTruthy()
  })
})
