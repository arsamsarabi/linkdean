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
  useRecordBookmarkOpen: () => vi.fn(),
  useOwnerActions: () => ({ following: false, muted: false, blocking: false }),
  useApiImage: () => ({ src: undefined, failed: false }),
}))

vi.mock('@/components/social', () => ({ SocialDrawer: () => null }))

const bookmark: BookmarkCardItem = {
  id: 'b1',
  url: 'https://mantine.dev/core/button',
  title: 'Mantine button',
  is_vault: false,
  tags: ['react', 'ui'],
  categories: [{ id: 'c1', name: 'Linkdean', path: 'Work / Projects / Linkdean' }],
  can_write: true,
}

function renderCard(item: BookmarkCardItem = bookmark, layout: 'card' | 'row' = 'card') {
  render(
    <MantineProvider env="test">
      <BookmarkCard layout={layout} bookmark={item} onEdit={vi.fn()} onDelete={vi.fn()} />
    </MantineProvider>
  )
}

describe('BookmarkCard folders and tags', () => {
  afterEach(cleanup)

  it('shows the folder path as breadcrumbs under the title', () => {
    renderCard()
    const folder = screen.getByRole('link', { name: /work.*projects.*linkdean/i })
    expect(folder.textContent).toBe('Work›Projects›Linkdean')
    const title = screen.getByText('Mantine button')
    expect(title.compareDocumentPosition(folder) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  })

  it('keeps tags behind a toggle on cards', async () => {
    renderCard()
    const toggle = screen.getByRole('button', { name: '2 tags' })
    expect(toggle.getAttribute('aria-expanded')).toBe('false')
    await userEvent.click(toggle)
    expect(toggle.getAttribute('aria-expanded')).toBe('true')
    expect(screen.getByText('react')).toBeTruthy()
    expect(screen.getByText('ui')).toBeTruthy()
  })

  it('opens tags in a popover on list rows', async () => {
    renderCard(bookmark, 'row')
    expect(screen.queryByText('react')).toBeNull()
    await userEvent.click(screen.getByRole('button', { name: '2 tags' }))
    expect(await screen.findByText('react')).toBeTruthy()
  })

  it('shows no toggle without tags', () => {
    renderCard({ ...bookmark, tags: [] })
    expect(screen.queryByRole('button', { name: /tags?$/ })).toBeNull()
  })
})
