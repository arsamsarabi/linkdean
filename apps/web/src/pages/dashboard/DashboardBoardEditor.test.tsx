import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterAll, afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import type { Bookmark } from '@/schemas'
import { defaultSectionSettings, type DashboardDraft } from '@/services'
import { DashboardBoardEditor } from './DashboardBoardEditor'

const bm = (id: string, title: string): Bookmark =>
  ({ id, title, url: `https://${id}.example`, is_vault: false }) as Bookmark

const bookmarks = [bm('a', 'Alpha'), bm('b', 'Beta')]

const draft: DashboardDraft = {
  title: 'Home',
  sections: [
    { ...defaultSectionSettings('pinned'), key: 'p', title: 'Daily', bookmark_ids: ['a', 'b'] },
    { ...defaultSectionSettings('latest'), key: 'l', title: 'New', bookmark_ids: [], width: 'half' },
  ],
}

function renderEditor(canAddSelfFilling = true) {
  const handlers = {
    onUpdateTitle: vi.fn(),
    onUpdateBookmarks: vi.fn(),
    onUpdateSection: vi.fn(),
    onMoveSection: vi.fn(),
    onMovePin: vi.fn(),
    onRemoveSection: vi.fn(),
    onAddSection: vi.fn(() => 'p'),
  }
  render(
    <MantineProvider env="test">
      <DashboardBoardEditor
        draft={draft}
        bookmarkOptions={bookmarks.map((b) => ({ value: b.id, label: b.title }))}
        bookmarkById={new Map(bookmarks.map((b) => [b.id, b]))}
        folderOptions={[]}
        tagOptions={[]}
        folderName={() => undefined}
        canAddSelfFilling={canAddSelfFilling}
        {...handlers}
      />
    </MantineProvider>
  )
  return handlers
}

describe('DashboardBoardEditor', () => {
  beforeAll(() => {
    // SegmentedControl measures itself; jsdom has no ResizeObserver.
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {}
        unobserve(): void {}
        disconnect(): void {}
      }
    )
  })
  afterAll(() => vi.unstubAllGlobals())
  afterEach(cleanup)

  it('gives every section and pinned bookmark a named drag handle', () => {
    renderEditor()
    expect(screen.getByRole('button', { name: 'Move section “Daily”' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Move section “New”' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Move bookmark “Alpha”' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Move bookmark “Beta”' })).toBeTruthy()
  })

  it('lays sections out at their width', () => {
    renderEditor()
    const cell = (name: string) => screen.getByRole('region', { name }).parentElement!
    expect(cell('Section “Daily”').dataset.width).toBe('full')
    expect(cell('Section “New”').dataset.width).toBe('half')
  })

  it('changes a section width from the board', async () => {
    const h = renderEditor()
    const daily = screen.getByRole('region', { name: 'Section “Daily”' })
    await userEvent.click(within(daily).getByText('Third'))
    expect(h.onUpdateSection).toHaveBeenCalledWith(0, { width: 'third' })
  })

  it('unpins a bookmark and removes a section', async () => {
    const h = renderEditor()
    await userEvent.click(screen.getByRole('button', { name: 'Unpin “Alpha”' }))
    expect(h.onUpdateBookmarks).toHaveBeenCalledWith(0, ['b'])
    await userEvent.click(screen.getByRole('button', { name: 'Remove “New”' }))
    expect(h.onRemoveSection).toHaveBeenCalledWith(1)
  })

  it('opens the settings drawer for a section', async () => {
    renderEditor()
    await userEvent.click(screen.getByRole('button', { name: 'Settings for “New”' }))
    const drawer = await screen.findByRole('dialog', { name: 'Latest section' })
    expect(within(drawer).getByLabelText('Title')).toHaveProperty('value', 'New')
    expect(within(drawer).getByLabelText('Bookmarks to show')).toBeTruthy()
  })

  it('offers only pinned sections when self-filling ones are not allowed', async () => {
    renderEditor(false)
    await userEvent.click(screen.getByRole('button', { name: 'Add section' }))
    const latest = await screen.findByRole('menuitem', { name: /Latest/ })
    expect(latest).toHaveProperty('disabled', true)
    expect(screen.getByRole('menuitem', { name: /Pinned/ })).toHaveProperty('disabled', false)
  })
})
