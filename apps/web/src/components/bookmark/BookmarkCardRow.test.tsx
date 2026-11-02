import type { ReactNode } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { BookmarkCard, type BookmarkCardItem } from './BookmarkCard'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, children, ...rest }: { to: string; children: ReactNode }) => (
    <a href={to} {...rest}>
      {children}
    </a>
  ),
  useNavigate: () => vi.fn(),
}))

vi.mock('@/hooks', () => ({
  useSaveBookmarkCopy: () => ({ mutateAsync: vi.fn() }),
  useOwnerActions: () => ({ following: false, muted: false, blocking: false }),
  useApiImage: () => ({ src: undefined, failed: false }),
}))

vi.mock('@/components/social', () => ({ SocialDrawer: () => null }))

const bookmark: BookmarkCardItem = {
  id: 'b1',
  url: 'https://mantine.dev/core/button',
  title: 'Mantine button',
  description: 'Docs for the button',
  is_vault: false,
  tags: [],
  categories: [{ id: 'c1', name: 'Design', path: 'Design' }],
  is_favorite: true,
  can_write: true,
}

function renderRow(props: Partial<Parameters<typeof BookmarkCard>[0]> = {}) {
  const handlers = { onEdit: vi.fn(), onDelete: vi.fn(), onToggleSelect: vi.fn() }
  render(
    <MantineProvider env="test">
      <BookmarkCard layout="row" bookmark={bookmark} {...handlers} {...props} />
    </MantineProvider>
  )
  return handlers
}

describe('BookmarkCard row layout', () => {
  afterEach(cleanup)

  it('shows the title link, site and description, and folders', () => {
    renderRow()
    expect(screen.getByRole('link', { name: /mantine button/i }).getAttribute('href')).toBe(bookmark.url)
    expect(screen.getByText('mantine.dev · Docs for the button')).toBeTruthy()
    expect(screen.getByText('Design')).toBeTruthy()
    expect(screen.queryByRole('link', { name: /open mantine button/i })).toBeNull()
  })

  it('keeps the bookmark menu and selection', async () => {
    const h = renderRow({ selectMode: true, selected: false })
    await userEvent.click(screen.getByRole('checkbox', { name: 'Select Mantine button' }))
    expect(h.onToggleSelect).toHaveBeenCalledWith('b1', false)
    await userEvent.click(screen.getByRole('button', { name: /actions|menu|more/i }))
    await userEvent.click(await screen.findByRole('menuitem', { name: /edit/i }))
    expect(h.onEdit).toHaveBeenCalledWith('b1')
  })
})
