import { buildCategoryTree, type SortKey } from '@/lib'
import type { Bookmark, Category, CategoryAccess, FolderCounts } from '@/schemas'

/** Virtual folder ids (not real category UUIDs). */
export const FOLDER_ALL = 'all' as const
export const FOLDER_UNCATEGORIZED = 'uncategorized' as const

export type ExplorerFolderId = typeof FOLDER_ALL | typeof FOLDER_UNCATEGORIZED | string

export type ExplorerFolder = {
  id: ExplorerFolderId
  name: string
  /** Bookmarks directly in the folder, from the server; `null` until the counts load. */
  bookmarkCount: number | null
  isVault: boolean
  isHidden: boolean
  icon?: string | null
  /** Effective access; omitted for virtual folders (All / Unsorted). */
  access?: CategoryAccess
  /** Owner has shared this folder (or an ancestor) out. */
  sharedOut?: boolean
  /** Owner-only: invitee labels for outgoing share tooltip. */
  sharedWith?: string[]
  /** Invitee-only: owner's display name for share badge. */
  ownerDisplayName?: string | null
  deletedAt?: string | null
  createdAt?: string | null
  updatedAt?: string | null
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

function filterVisibleCats(cats: readonly Category[], showHidden: boolean): Category[] {
  return cats.filter((c) => showHidden || !c.is_hidden)
}

function toFolder(cat: Category, counts: FolderCounts | undefined): ExplorerFolder {
  return {
    id: cat.id,
    name: cat.name,
    bookmarkCount: counts ? (counts.folders[cat.id] ?? 0) : null,
    isVault: cat.is_vault,
    isHidden: cat.is_hidden,
    icon: cat.icon,
    access: cat.access,
    sharedOut: cat.shared_out,
    sharedWith: cat.shared_with,
    ownerDisplayName: cat.owner_display_name,
    deletedAt: cat.deleted_at,
    createdAt: cat.created_at ?? null,
    updatedAt: cat.updated_at ?? null,
  }
}

/** Top-level folders at All, plus Unsorted. */
function rootFolders(
  cats: readonly Category[],
  counts: FolderCounts | undefined,
  showHidden: boolean
): ExplorerFolder[] {
  const visible = filterVisibleCats(cats, showHidden)
  const roots = buildCategoryTree(visible).map((n) => toFolder(n.cat, counts))
  return [
    ...roots,
    {
      id: FOLDER_UNCATEGORIZED,
      name: 'Unsorted',
      bookmarkCount: counts ? counts.uncategorized : null,
      isVault: false,
      isHidden: false,
    },
  ]
}

/** Contents of the selected folder for the drill-down pane. Tile counts come from `counts`. */
export function listFolderContents(
  folderId: ExplorerFolderId | undefined,
  cats: readonly Category[],
  bookmarks: readonly Bookmark[],
  showHidden = false,
  counts?: FolderCounts
): ExplorerContents {
  const id = folderId && folderId.length > 0 ? folderId : FOLDER_ALL
  const visible = filterVisibleCats(cats, showHidden)

  if (id === FOLDER_ALL) {
    return {
      folders: rootFolders(visible, counts, showHidden),
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
  const folders: ExplorerFolder[] = byParent.map((c) => toFolder(c, counts))

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
    return [home, { id: FOLDER_UNCATEGORIZED, name: 'Unsorted' }]
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

/** Sort folders by library sort key. Unsorted stays last. */
export function sortExplorerFolders(folders: readonly ExplorerFolder[], sort: SortKey): ExplorerFolder[] {
  const uncategorized = folders.filter((f) => f.id === FOLDER_UNCATEGORIZED)
  const rest = folders.filter((f) => f.id !== FOLDER_UNCATEGORIZED)
  const byNameAsc = (a: ExplorerFolder, b: ExplorerFolder) => a.name.localeCompare(b.name)
  const byNameDesc = (a: ExplorerFolder, b: ExplorerFolder) => b.name.localeCompare(a.name)
  const updatedOf = (f: ExplorerFolder) => f.updatedAt || f.createdAt || ''
  const createdOf = (f: ExplorerFolder) => f.createdAt || ''
  const cmp = (a: ExplorerFolder, b: ExplorerFolder): number => {
    switch (sort) {
      case 'title_desc':
        return byNameDesc(a, b)
      case 'title_asc':
        return byNameAsc(a, b)
      case 'updated_asc':
        return updatedOf(a).localeCompare(updatedOf(b)) || byNameAsc(a, b)
      case 'created_desc':
        return createdOf(b).localeCompare(createdOf(a)) || byNameAsc(a, b)
      case 'updated_desc':
      default:
        return updatedOf(b).localeCompare(updatedOf(a)) || byNameAsc(a, b)
    }
  }
  return [...rest].sort(cmp).concat(uncategorized)
}
