import { MantineProvider } from '@mantine/core'
import { cleanup, render } from '@testing-library/react'
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
