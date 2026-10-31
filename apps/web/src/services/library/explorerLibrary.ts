import { buildCategoryTree, type CatNode } from '@/lib'
import type { Bookmark, Category } from '@/schemas'

/** Virtual folder ids (not real category UUIDs). */
export const FOLDER_ALL = 'all' as const
export const FOLDER_UNCATEGORIZED = 'uncategorized' as const

export type ExplorerFolderId = typeof FOLDER_ALL | typeof FOLDER_UNCATEGORIZED | string

export type ExplorerTreeNode = {
  id: ExplorerFolderId
  name: string
  bookmarkCount: number
  isVault: boolean
  isHidden: boolean
  children: ExplorerTreeNode[]
}

export type ExplorerFolder = {
  id: ExplorerFolderId
  name: string
  bookmarkCount: number
  isVault: boolean
  isHidden: boolean
}

export type ExplorerContents = {
  folders: ExplorerFolder[]
  bookmarks: Bookmark[]
}

export type BreadcrumbSegment = {
  id: ExplorerFolderId
  name: string
}

export type ExplorerBookmarkRef = {
  id: string
  category_id?: string | null
  categories?: readonly { id: string }[]
}

function categoryIdsOf(b: ExplorerBookmarkRef): Set<string> {
  const ids = new Set<string>()
  if (b.category_id) ids.add(b.category_id)
  for (const c of b.categories || []) ids.add(c.id)
  return ids
}

export function bookmarkInFolder(b: ExplorerBookmarkRef, folderId: string): boolean {
  return categoryIdsOf(b).has(folderId)
}

export function isUncategorized(b: ExplorerBookmarkRef): boolean {
  return categoryIdsOf(b).size === 0
}

function countDirectBookmarks(folderId: string, bookmarks: readonly ExplorerBookmarkRef[]): number {
  return bookmarks.filter((b) => bookmarkInFolder(b, folderId)).length
}

function filterVisibleCats(cats: readonly Category[], showHidden: boolean): Category[] {
  return cats.filter((c) => showHidden || !c.is_hidden)
}

function mapTree(
  nodes: CatNode[],
  bookmarks: readonly ExplorerBookmarkRef[],
  showHidden: boolean
): ExplorerTreeNode[] {
  return nodes
    .filter((n) => showHidden || !n.cat.is_hidden)
    .map((n) => ({
      id: n.cat.id,
      name: n.cat.name,
      bookmarkCount: countDirectBookmarks(n.cat.id, bookmarks),
      isVault: n.cat.is_vault,
      isHidden: n.cat.is_hidden,
      children: mapTree(n.children, bookmarks, showHidden),
    }))
}

/** Category tree for the left pane, plus virtual Uncategorized under All. */
export function buildExplorerTree(
  cats: readonly Category[],
  bookmarks: readonly ExplorerBookmarkRef[],
  showHidden = false
): ExplorerTreeNode[] {
  const visible = filterVisibleCats(cats, showHidden)
  const roots = mapTree(buildCategoryTree(visible), bookmarks, showHidden)
  const uncategorizedCount = bookmarks.filter(isUncategorized).length

  return [
    {
      id: FOLDER_ALL,
      name: 'All',
      bookmarkCount: bookmarks.length,
      isVault: false,
      isHidden: false,
      children: [
        ...roots,
        {
          id: FOLDER_UNCATEGORIZED,
          name: 'Uncategorized',
          bookmarkCount: uncategorizedCount,
          isVault: false,
          isHidden: false,
          children: [],
        },
      ],
    },
  ]
}

function toFolder(n: ExplorerTreeNode): ExplorerFolder {
  return {
    id: n.id,
    name: n.name,
    bookmarkCount: n.bookmarkCount,
    isVault: n.isVault,
    isHidden: n.isHidden,
  }
}

function rootFolders(
  cats: readonly Category[],
  bookmarks: readonly ExplorerBookmarkRef[],
  showHidden: boolean
): ExplorerFolder[] {
  const tree = buildExplorerTree(cats, bookmarks, showHidden)
  const all = tree[0]
  if (!all) return []
  return all.children.map(toFolder)
}

/** Contents of the selected folder for the right pane. */
export function listFolderContents(
  folderId: ExplorerFolderId | undefined,
  cats: readonly Category[],
  bookmarks: readonly Bookmark[],
  showHidden = false
): ExplorerContents {
  const id = folderId && folderId.length > 0 ? folderId : FOLDER_ALL
  const visible = filterVisibleCats(cats, showHidden)

  if (id === FOLDER_ALL) {
    return {
      folders: rootFolders(visible, bookmarks, showHidden),
      bookmarks: [],
    }
  }

  if (id === FOLDER_UNCATEGORIZED) {
    return {
      folders: [],
      bookmarks: bookmarks.filter(isUncategorized),
    }
  }

  const byParent = visible.filter((c) => c.parent_id === id)
  const folders: ExplorerFolder[] = byParent.map((c) => ({
    id: c.id,
    name: c.name,
    bookmarkCount: countDirectBookmarks(c.id, bookmarks),
    isVault: c.is_vault,
    isHidden: c.is_hidden,
  }))

  return {
    folders,
    bookmarks: bookmarks.filter((b) => bookmarkInFolder(b, id)),
  }
}

/** Breadcrumb from All → … → current folder. */
export function folderBreadcrumb(
  folderId: ExplorerFolderId | undefined,
  byId: Map<string, Category>
): BreadcrumbSegment[] {
  const id = folderId && folderId.length > 0 ? folderId : FOLDER_ALL
  const home: BreadcrumbSegment = { id: FOLDER_ALL, name: 'All' }

  if (id === FOLDER_ALL) return [home]

  if (id === FOLDER_UNCATEGORIZED) {
    return [home, { id: FOLDER_UNCATEGORIZED, name: 'Uncategorized' }]
  }

  const parts: BreadcrumbSegment[] = []
  let cur: string | null | undefined = id
  let guard = 0
  while (cur) {
    guard += 1
    if (guard > 64) break
    const cat = byId.get(cur)
    if (!cat) break
    parts.push({ id: cat.id, name: cat.name })
    cur = cat.parent_id
  }
  parts.reverse()
  return [home, ...parts]
}

export function normalizeFolderId(raw: unknown): ExplorerFolderId | undefined {
  if (typeof raw !== 'string' || !raw.trim()) return undefined
  const v = raw.trim()
  if (v === FOLDER_ALL) return undefined
  return v
}
