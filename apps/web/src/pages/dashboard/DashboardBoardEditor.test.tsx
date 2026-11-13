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
    {
      ...defaultSectionSettings('pinned'),
      key: 'p',
      title: 'Daily',
      bookmark_ids: ['a', 'b'],
      layout: { x: 0, y: 0, w: 12, h: 11 },
    },
    {
      ...defaultSectionSettings('latest'),
      key: 'l',
      title: 'New',
      bookmark_ids: [],
      layout: { x: 0, y: 11, w: 6, h: 8 },
    },
  ],
}

function renderEditor(canAddSelfFilling = true) {
  const handlers = {
    onUpdateTitle: vi.fn(),
    onUpdateBookmarks: vi.fn(),
    onUpdateSection: vi.fn(),
    onApplyLayout: vi.fn(),
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

  it('gives every section and pinned bookmark a named handle', () => {
    renderEditor()
    expect(screen.getByRole('button', { name: 'Move or resize section “Daily”' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Move or resize section “New”' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Move bookmark “Alpha”' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Move bookmark “Beta”' })).toBeTruthy()
  })

  it('shows each section size in columns by rows', () => {
    renderEditor()
    expect(within(screen.getByRole('region', { name: 'Section “New”' })).getByText('6 × 8')).toBeTruthy()
  })

  it('moves and resizes a section from the keyboard, and says so', async () => {
    const h = renderEditor()
    const handle = screen.getByRole('button', { name: 'Move or resize section “New”' })
    handle.focus()
    await userEvent.keyboard('{Shift>}{ArrowRight}{/Shift}')
    expect(h.onApplyLayout).toHaveBeenLastCalledWith(
      new Map([
        ['p', { x: 0, y: 0, w: 12, h: 11 }],
        ['l', { x: 0, y: 11, w: 7, h: 8 }],
      ])
    )
    expect(screen.getByText('“New” is 7 columns wide and 8 rows tall.').getAttribute('aria-live')).toBe('polite')
    await userEvent.keyboard('{ArrowUp}')
    const placed = h.onApplyLayout.mock.lastCall![0] as Map<string, { y: number }>
    expect(placed.get('l')!.y).toBe(0)
    expect(placed.get('p')!.y).toBe(8)
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
