import { buildCategoryTree, type SortKey } from '@/lib'
import type { Bookmark, Category, CategoryAccess } from '@/schemas'

/** Virtual folder ids (not real category UUIDs). */
export const FOLDER_ALL = 'all' as const
export const FOLDER_UNCATEGORIZED = 'uncategorized' as const

export type ExplorerFolderId = typeof FOLDER_ALL | typeof FOLDER_UNCATEGORIZED | string

export type ExplorerFolder = {
  id: ExplorerFolderId
  name: string
  bookmarkCount: number
  isVault: boolean
  isHidden: boolean
  /** Effective access; omitted for virtual folders (All / Uncategorized). */
  access?: CategoryAccess
  /** Owner has shared this folder (or an ancestor) out. */
  sharedOut?: boolean
}

export function isSharedFolder(folder: ExplorerFolder): boolean {
  return folder.access === 'read' || folder.access === 'write'
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

function toFolder(cat: Category, bookmarks: readonly ExplorerBookmarkRef[]): ExplorerFolder {
  return {
    id: cat.id,
    name: cat.name,
    bookmarkCount: countDirectBookmarks(cat.id, bookmarks),
    isVault: cat.is_vault,
    isHidden: cat.is_hidden,
    access: cat.access,
    sharedOut: cat.shared_out,
  }
}

/** Top-level folders at All, plus Uncategorized. */
function rootFolders(
  cats: readonly Category[],
  bookmarks: readonly ExplorerBookmarkRef[],
  showHidden: boolean
): ExplorerFolder[] {
  const visible = filterVisibleCats(cats, showHidden)
  const roots = buildCategoryTree(visible).map((n) => toFolder(n.cat, bookmarks))
  return [
    ...roots,
    {
      id: FOLDER_UNCATEGORIZED,
      name: 'Uncategorized',
      bookmarkCount: bookmarks.filter(isUncategorized).length,
      isVault: false,
      isHidden: false,
    },
  ]
}

/** Contents of the selected folder for the drill-down pane. */
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
  const folders: ExplorerFolder[] = byParent.map((c) => toFolder(c, bookmarks))

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

/** Sort folders by library sort key. Uncategorized stays last. Date sorts → name (folders have no timestamps). */
export function sortExplorerFolders(folders: readonly ExplorerFolder[], sort: SortKey): ExplorerFolder[] {
  const uncategorized = folders.filter((f) => f.id === FOLDER_UNCATEGORIZED)
  const rest = folders.filter((f) => f.id !== FOLDER_UNCATEGORIZED)
  const byNameAsc = (a: ExplorerFolder, b: ExplorerFolder) => a.name.localeCompare(b.name)
  const byNameDesc = (a: ExplorerFolder, b: ExplorerFolder) => b.name.localeCompare(a.name)
  const cmp = sort === 'title_desc' ? byNameDesc : byNameAsc
  return [...rest].sort(cmp).concat(uncategorized)
}
