import { MantineProvider } from '@mantine/core'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Bookmark } from '@/schemas'
import { FilingQueuePage } from './FilingQueuePage'

const bm = (id: string, title: string): Bookmark =>
  ({ id, title, url: `https://site.example/${id}`, tags: [], is_favorite: false }) as unknown as Bookmark

const state = vi.hoisted(() => ({
  items: [] as Bookmark[],
  update: vi.fn(),
  remove: vi.fn(),
}))

vi.mock('@tanstack/react-router', () => ({ useNavigate: () => vi.fn() }))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))
vi.mock('@/components/bookmark/BookmarkCardPreview', () => ({ BookmarkCardPreview: () => null }))
vi.mock('@/components/bookmark/BookmarkCategoriesField', () => ({
  BookmarkCategoriesField: ({ value }: { value: string[] }) => <p>folders: {value.join(',') || 'none'}</p>,
}))
vi.mock('@/hooks', () => ({
  useUnsortedQueue: () => ({
    isPending: false,
    isError: false,
    data: { pages: [{ items: state.items, total: state.items.length, next_cursor: null }] },
    hasNextPage: false,
    isFetchingNextPage: false,
    fetchNextPage: vi.fn(),
  }),
  useCategories: () => ({
    isPending: false,
    data: [{ id: 'f1', name: 'Rust', parent_id: null, is_vault: false, deleted_at: null }],
  }),
  useFilingSuggestions: () => ({ data: { folders: [{ id: 'f1', count: 2 }], tags: [{ name: 'lang', count: 2 }] } }),
  useUpdateBookmark: () => ({ isPending: false, mutate: state.update }),
  useDeleteBookmark: () => ({ isPending: false, mutate: state.remove }),
}))

beforeAll(() => {
  // Mantine's inputs measure themselves; jsdom has no ResizeObserver.
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})

function renderPage(): void {
  render(
    <MantineProvider>
      <FilingQueuePage />
    </MantineProvider>
  )
}

const fileButton = (): HTMLButtonElement => screen.getByRole('button', { name: 'File' })

describe('FilingQueuePage', () => {
  beforeEach(() => {
    state.items = [bm('a', 'First'), bm('b', 'Second')]
    state.update = vi.fn((_vars, opts: { onSuccess: () => void }) => opts.onSuccess())
    state.remove = vi.fn((_id, opts: { onSuccess: () => void }) => opts.onSuccess())
  })
  afterEach(cleanup)

  it('shows the newest card and only files once a folder is chosen', async () => {
    renderPage()
    expect((screen.getByRole('textbox', { name: 'Title' }) as HTMLInputElement).value).toBe('First')
    expect(screen.getByText(/2 left/)).toBeTruthy()
    expect(fileButton().disabled).toBe(true)
    await userEvent.click(screen.getByRole('button', { name: /Rust/ }))
    expect(screen.getByText('folders: f1')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: '+ lang' }))
    await userEvent.click(fileButton())
    expect(state.update).toHaveBeenCalledWith(
      { id: 'a', body: { category_ids: ['f1'], tags: ['lang'], is_favorite: false } },
      expect.anything()
    )
    expect((screen.getByRole('textbox', { name: 'Title' }) as HTMLInputElement).value).toBe('Second')
  })

  it('works from the keyboard: number picks a folder, Enter files, S skips, D deletes', () => {
    state.items = [bm('a', 'First'), bm('b', 'Second'), bm('c', 'Third')]
    renderPage()
    fireEvent.keyDown(window, { key: 'Enter' })
    expect(state.update).not.toHaveBeenCalled()
    fireEvent.keyDown(document.body, { key: '1' })
    fireEvent.keyDown(document.body, { key: 'Enter' })
    expect(state.update).toHaveBeenCalledTimes(1)
    fireEvent.keyDown(document.body, { key: 's' })
    expect((screen.getByRole('textbox', { name: 'Title' }) as HTMLInputElement).value).toBe('Third')
    fireEvent.keyDown(document.body, { key: 'd' })
    expect(state.remove).toHaveBeenCalledWith('c', expect.anything())
    expect(screen.getByText('All done')).toBeTruthy()
  })

  it('ignores shortcut letters typed into the title', async () => {
    renderPage()
    const title = screen.getByRole('textbox', { name: 'Title' })
    await userEvent.type(title, ' sd')
    expect((title as HTMLInputElement).value).toBe('First sd')
    expect(state.remove).not.toHaveBeenCalled()
  })

  it('says when there is nothing to file', () => {
    state.items = []
    renderPage()
    expect(screen.getByText('Nothing to file')).toBeTruthy()
  })
})
