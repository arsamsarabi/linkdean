import { describe, expect, it } from 'vitest'
import type { Category } from '@/schemas'
import { filterFolders, recentFolders } from './folderLists'

function cat(id: string, updated_at: string, parent_id: string | null = null): Category {
  return { id, name: id, parent_id, updated_at } as Category
}

const cats = [
  cat('Design', '2026-01-04'),
  cat('Fonts', '2026-01-01', 'Design'),
  cat('Recipes', '2026-01-03'),
  cat('Travel', '2026-01-05'),
  cat('Work', '2026-01-02'),
]

describe('recentFolders', () => {
  it('picks the three most recently updated', () => {
    expect(recentFolders(cats, undefined).map((c) => c.id)).toEqual(['Travel', 'Design', 'Recipes'])
  })

  it('adds the open folder when it is not among them', () => {
    expect(recentFolders(cats, 'Fonts').map((c) => c.id)).toEqual(['Travel', 'Design', 'Recipes', 'Fonts'])
    expect(recentFolders(cats, 'Design')).toHaveLength(3)
  })
})

describe('filterFolders', () => {
  it('matches names at any depth, ignoring case, with their path', () => {
    expect(filterFolders(cats, 'FON')).toEqual([{ cat: cats[1], path: 'Design / Fonts' }])
    expect(filterFolders(cats, 'r').map((m) => m.path)).toEqual(['Recipes', 'Travel', 'Work'])
    expect(filterFolders(cats, '  ')).toEqual([])
  })
})
