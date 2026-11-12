import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ExplorerFolder } from '@/services'
import { ExplorerFolderTile } from './ExplorerFolderTile'

afterEach(cleanup)

const folder = (bookmarkCount: number | null): ExplorerFolder => ({
  id: 'c1',
  name: 'Work',
  bookmarkCount,
  isVault: false,
  isHidden: false,
})

function tileName(count: number | null): string {
  render(
    <MantineProvider>
      <ExplorerFolderTile folder={folder(count)} onOpen={vi.fn()} />
    </MantineProvider>
  )
  return screen.getByRole('button').getAttribute('aria-label') ?? ''
}

describe('ExplorerFolderTile', () => {
  it.each([
    [0, 'Open folder Work, 0 bookmarks'],
    [1, 'Open folder Work, 1 bookmark'],
    [12, 'Open folder Work, 12 bookmarks'],
    [null, 'Open folder Work'],
  ])('labels %s as %s', (count, label) => {
    expect(tileName(count)).toBe(label)
  })
})
