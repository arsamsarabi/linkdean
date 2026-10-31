import { describe, expect, it } from 'vitest'
import type { Bookmark, Category } from '@/schemas'
import {
  FOLDER_ALL,
  FOLDER_UNCATEGORIZED,
  buildExplorerTree,
  folderBreadcrumb,
  listFolderContents,
  normalizeFolderId,
} from './explorerLibrary'

const cats: Category[] = [
  { id: 'root', parent_id: null, name: 'Work', is_hidden: false, is_vault: false, position: 0 },
  { id: 'child', parent_id: 'root', name: 'Docs', is_hidden: false, is_vault: false, position: 1 },
  { id: 'hidden', parent_id: null, name: 'Secret', is_hidden: true, is_vault: true, position: 2 },
]

const baseBm = {
  description: null,
  notes: null,
  favicon_url: null,
  preview_image_url: null,
  has_preview: false,
  is_vault: false,
  tags: [] as string[],
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-02T00:00:00Z',
}

const bookmarks: Bookmark[] = [
  {
    ...baseBm,
    id: 'b1',
    url: 'https://a.example',
    title: 'In root',
    category_id: 'root',
    categories: [{ id: 'root', name: 'Work', path: 'Work' }],
  },
  {
    ...baseBm,
    id: 'b2',
    url: 'https://b.example',
    title: 'In child',
    category_id: 'child',
    categories: [{ id: 'child', name: 'Docs', path: 'Work / Docs' }],
  },
  {
    ...baseBm,
    id: 'b3',
    url: 'https://c.example',
    title: 'Loose',
    category_id: null,
    categories: [],
  },
]

describe('buildExplorerTree', () => {
  it('wraps roots under All and adds Uncategorized', () => {
    const tree = buildExplorerTree(cats, bookmarks, false)
    expect(tree).toHaveLength(1)
    expect(tree[0]?.id).toBe(FOLDER_ALL)
    expect(tree[0]?.bookmarkCount).toBe(3)
    const childIds = tree[0]?.children.map((c) => c.id)
    expect(childIds).toContain('root')
    expect(childIds).toContain(FOLDER_UNCATEGORIZED)
    expect(childIds).not.toContain('hidden')
    expect(tree[0]?.children.find((c) => c.id === FOLDER_UNCATEGORIZED)?.bookmarkCount).toBe(1)
    expect(tree[0]?.children.find((c) => c.id === 'root')?.bookmarkCount).toBe(1)
    expect(tree[0]?.children.find((c) => c.id === 'root')?.children[0]?.id).toBe('child')
  })

  it('includes hidden when showHidden', () => {
    const tree = buildExplorerTree(cats, bookmarks, true)
    expect(tree[0]?.children.map((c) => c.id)).toContain('hidden')
  })
})

describe('listFolderContents', () => {
  it('lists top folders at All, no bookmarks', () => {
    const c = listFolderContents(FOLDER_ALL, cats, bookmarks, false)
    expect(c.bookmarks).toHaveLength(0)
    expect(c.folders.map((f) => f.id)).toEqual(['root', FOLDER_UNCATEGORIZED])
  })

  it('lists child folders and bookmarks for a category', () => {
    const c = listFolderContents('root', cats, bookmarks, false)
    expect(c.folders.map((f) => f.id)).toEqual(['child'])
    expect(c.bookmarks.map((b) => b.id)).toEqual(['b1'])
  })

  it('lists uncategorized bookmarks', () => {
    const c = listFolderContents(FOLDER_UNCATEGORIZED, cats, bookmarks, false)
    expect(c.folders).toHaveLength(0)
    expect(c.bookmarks.map((b) => b.id)).toEqual(['b3'])
  })
})

describe('folderBreadcrumb', () => {
  it('builds path from All', () => {
    const byId = new Map(cats.map((c) => [c.id, c]))
    expect(folderBreadcrumb('child', byId)).toEqual([
      { id: FOLDER_ALL, name: 'All' },
      { id: 'root', name: 'Work' },
      { id: 'child', name: 'Docs' },
    ])
    expect(folderBreadcrumb(FOLDER_UNCATEGORIZED, byId)).toEqual([
      { id: FOLDER_ALL, name: 'All' },
      { id: FOLDER_UNCATEGORIZED, name: 'Uncategorized' },
    ])
    expect(folderBreadcrumb(undefined, byId)).toEqual([{ id: FOLDER_ALL, name: 'All' }])
  })
})

describe('normalizeFolderId', () => {
  it('treats all/empty as undefined', () => {
    expect(normalizeFolderId('all')).toBeUndefined()
    expect(normalizeFolderId('')).toBeUndefined()
    expect(normalizeFolderId('root')).toBe('root')
  })
})
