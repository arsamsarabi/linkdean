import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { LibraryPageHeader } from './LibraryPageHeader'

const lib = vi.hoisted(() => ({
  loading: false,
  viewMode: 'grid',
  searching: false,
  selectMode: false,
  canCreateInContext: true,
  list: [{ id: 'b1' }],
  filterFlags: ['favorite'],
  toggleFlagFilter: vi.fn(),
  onViewModeChange: vi.fn(),
  enterSelectMode: vi.fn(),
  exitSelectMode: vi.fn(),
  openCreate: vi.fn(),
}))

vi.mock('./useLibraryPageModel', () => ({ useLibraryPageModel: () => lib }))

function renderHeader(): void {
  render(
    <MantineProvider>
      <LibraryPageHeader />
    </MantineProvider>
  )
}

describe('LibraryPageHeader', () => {
  beforeEach(() => vi.clearAllMocks())
  afterEach(cleanup)

  it('places the quick filters before the view switch', () => {
    renderHeader()
    const names = screen
      .getAllByRole('button')
      .map((b) => b.textContent)
      .filter(Boolean)
    expect(names.slice(0, 6)).toEqual(['Reading list', 'Favorites', 'Pinned', 'Grid', 'List', 'Explorer'])
    expect(screen.queryByRole('button', { name: /duplicate/i })).toBeNull()
  })

  it('marks the active filter and view as pressed', () => {
    renderHeader()
    expect(screen.getByRole('button', { name: 'Favorites' }).getAttribute('aria-pressed')).toBe('true')
    expect(screen.getByRole('button', { name: 'Pinned' }).getAttribute('aria-pressed')).toBe('false')
    const view = within(screen.getByRole('group', { name: 'Library view' }))
    expect(view.getByRole('button', { name: 'Grid' }).getAttribute('aria-pressed')).toBe('true')
  })

  it('switches view and toggles filters', async () => {
    renderHeader()
    await userEvent.click(screen.getByRole('button', { name: 'Explorer' }))
    expect(lib.onViewModeChange).toHaveBeenCalledWith('explorer')
    await userEvent.click(screen.getByRole('button', { name: 'Pinned' }))
    expect(lib.toggleFlagFilter).toHaveBeenCalledWith('pinned')
  })
})
