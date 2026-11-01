import { describe, expect, it } from 'vitest'
import type { Bookmark, Category } from '@/schemas'
import {
  FOLDER_ALL,
  FOLDER_UNCATEGORIZED,
  folderBreadcrumb,
  isSharedFolder,
  listFolderContents,
  normalizeFolderId,
  sortExplorerFolders,
  type ExplorerFolder,
} from './explorerLibrary'

const cats: Category[] = [
  {
    id: 'root',
    parent_id: null,
    name: 'Work',
    is_hidden: false,
    is_vault: false,
    position: 0,
    access: 'owner',
    shared_out: false,
    shared_with: [],
  },
  {
    id: 'child',
    parent_id: 'root',
    name: 'Docs',
    is_hidden: false,
    is_vault: false,
    position: 1,
    access: 'owner',
    shared_out: false,
    shared_with: [],
  },
  {
    id: 'hidden',
    parent_id: null,
    name: 'Secret',
    is_hidden: true,
    is_vault: true,
    position: 2,
    access: 'owner',
    shared_out: false,
    shared_with: [],
  },
  {
    id: 'shared',
    parent_id: null,
    name: 'Team',
    is_hidden: false,
    is_vault: false,
    position: 3,
    access: 'read',
    shared_out: false,
    shared_with: [],
  },
]

const baseBm = {
  owner_user_id: 'u1',
  description: null,
  notes: null,
  favicon_url: null,
  preview_image_url: null,
  has_preview: false,
  is_vault: false,
  can_write: true,
  shared_out: false,
  shared_with: [] as string[],
  is_favorite: false,
  is_unread: false,
  is_pinned: false,
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

describe('listFolderContents', () => {
  it('lists top folders at All, no bookmarks; hides hidden cats', () => {
    const c = listFolderContents(FOLDER_ALL, cats, bookmarks, false)
    expect(c.bookmarks).toHaveLength(0)
    expect(c.folders.map((f) => f.id)).toEqual(['root', 'shared', FOLDER_UNCATEGORIZED])
    expect(c.folders.find((f) => f.id === FOLDER_UNCATEGORIZED)?.bookmarkCount).toBe(1)
    expect(c.folders.find((f) => f.id === 'root')?.bookmarkCount).toBe(1)
    expect(c.folders.find((f) => f.id === 'root')?.access).toBe('owner')
    expect(c.folders.find((f) => f.id === 'shared')?.access).toBe('read')
    expect(isSharedFolder(c.folders.find((f) => f.id === 'shared')!)).toBe(true)
  })

  it('includes hidden when showHidden', () => {
    const c = listFolderContents(FOLDER_ALL, cats, bookmarks, true)
    expect(c.folders.map((f) => f.id)).toContain('hidden')
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
      { id: FOLDER_UNCATEGORIZED, name: 'Unsorted' },
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

describe('sortExplorerFolders', () => {
  const folders: ExplorerFolder[] = [
    { id: 'z', name: 'Zebra', bookmarkCount: 0, isVault: false, isHidden: false },
    { id: 'a', name: 'Alpha', bookmarkCount: 0, isVault: false, isHidden: false },
    { id: FOLDER_UNCATEGORIZED, name: 'Unsorted', bookmarkCount: 1, isVault: false, isHidden: false },
  ]

  it('sorts by name A–Z and keeps Unsorted last', () => {
    expect(sortExplorerFolders(folders, 'title_asc').map((f) => f.id)).toEqual([
      'a',
      'z',
      FOLDER_UNCATEGORIZED,
    ])
  })

  it('sorts by name Z–A and keeps Unsorted last', () => {
    expect(sortExplorerFolders(folders, 'title_desc').map((f) => f.id)).toEqual([
      'z',
      'a',
      FOLDER_UNCATEGORIZED,
    ])
  })

  it('date sorts fall back to name asc', () => {
    expect(sortExplorerFolders(folders, 'updated_desc').map((f) => f.id)).toEqual([
      'a',
      'z',
      FOLDER_UNCATEGORIZED,
    ])
  })
})
