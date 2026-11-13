import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Bookmark } from '@/schemas'
import { DashboardSectionItems } from './DashboardSectionItems'

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useRecordBookmarkOpen: () => vi.fn(),
}))

const items = Array.from({ length: 11 }, (_, i) => ({
  bookmark: { id: `b${i}`, title: `B${i}`, url: `https://b${i}.example`, is_vault: false } as Bookmark,
  open_count: 11 - i,
}))

function keycaps(kind: 'pinned' | 'most_opened'): string[] {
  const { container } = render(
    <MantineProvider env="test">
      <DashboardSectionItems kind={kind} display="icons" items={items} viewerUserId={undefined} />
    </MantineProvider>
  )
  return Array.from(container.querySelectorAll('kbd'), (k) => k.textContent ?? '')
}

describe('DashboardSectionItems keycaps', () => {
  afterEach(cleanup)

  it('numbers the first nine bookmarks with the keys that open them', () => {
    expect(keycaps('pinned')).toEqual(['1', '2', '3', '4', '5', '6', '7', '8', '9'])
  })

  it('lets the rank stand for the key in a ranked section', () => {
    expect(keycaps('most_opened')).toEqual([])
  })
})

describe('DashboardSectionItems cards', () => {
  afterEach(cleanup)

  function renderCards(display: 'medium' | 'large') {
    const cards = [
      {
        bookmark: {
          id: 'c1',
          title: 'Iranian UK',
          url: 'https://iranianuk.com/news',
          description: 'News and culture',
          is_vault: false,
          tags: ['news', 'culture'],
          categories: [{ id: 'f1', name: 'News', path: 'Reading / News' }],
        } as unknown as Bookmark,
        open_count: 4,
      },
    ]
    render(
      <MantineProvider env="test">
        <DashboardSectionItems kind="most_opened" display={display} items={cards} viewerUserId={undefined} />
      </MantineProvider>
    )
  }

  it('shows rank and opens in the card body, as one link, without tags or folders', () => {
    renderCards('medium')
    const card = screen.getByRole('link', { name: /iranian uk/i })
    expect(card.getAttribute('href')).toBe('https://iranianuk.com/news')
    expect(within(card).getByText('#1')).toBeTruthy()
    expect(within(card).getByText('4 opens')).toBeTruthy()
    expect(within(card).getByText('iranianuk.com')).toBeTruthy()
    expect(screen.queryByText('news')).toBeNull()
    expect(screen.queryByText(/reading/i)).toBeNull()
    expect(screen.queryByText('News and culture')).toBeNull()
  })

  it('adds two lines of description on large cards', () => {
    renderCards('large')
    expect(screen.getByText('News and culture')).toBeTruthy()
  })
})
