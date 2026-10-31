export type PaletteItemKind = 'add' | 'search' | 'jump-category' | 'jump-all' | 'jump-uncategorized'

export type PaletteItem = {
  id: string
  kind: PaletteItemKind
  label: string
  description?: string
  /** Set for jump-category */
  categoryId?: string
}

export type PaletteCategory = {
  id: string
  label: string
  is_hidden?: boolean
}

function matches(hay: string, query: string): boolean {
  const q = query.trim().toLowerCase()
  if (!q) return true
  return hay.toLowerCase().includes(q)
}

function actionMatches(query: string, keywords: string[]): boolean {
  const q = query.trim().toLowerCase()
  if (!q) return true
  return keywords.some((k) => k.includes(q) || q.includes(k))
}

/** Ranked palette rows for query — actions first, then folder jumps. */
export function buildPaletteItems(
  query: string,
  categories: readonly PaletteCategory[],
  opts?: { showHidden?: boolean; categoryLimit?: number }
): PaletteItem[] {
  const showHidden = opts?.showHidden ?? false
  const categoryLimit = opts?.categoryLimit ?? 8
  const q = query.trim()
  const items: PaletteItem[] = []

  if (actionMatches(q, ['add', 'new', 'create', 'bookmark', 'link'])) {
    items.push({
      id: 'action:add',
      kind: 'add',
      label: 'Add bookmark',
      description: 'Open create drawer',
    })
  }

  if (actionMatches(q, ['search', 'find', 'query', q.toLowerCase()])) {
    items.push({
      id: 'action:search',
      kind: 'search',
      label: q ? `Search library for “${q}”` : 'Search library',
      description: q ? 'Jump to library with this query' : 'Focus library search',
    })
  }

  if (actionMatches(q, ['all', 'root', 'library', 'bookmarks'])) {
    items.push({
      id: 'jump:all',
      kind: 'jump-all',
      label: 'All bookmarks',
      description: 'Library root',
    })
  }

  if (actionMatches(q, ['uncategorized', 'unsorted', 'none'])) {
    items.push({
      id: 'jump:uncategorized',
      kind: 'jump-uncategorized',
      label: 'Uncategorized',
      description: 'Bookmarks with no folder',
    })
  }

  const cats = categories
    .filter((c) => showHidden || !c.is_hidden)
    .filter((c) => matches(c.label, q))
    .slice(0, categoryLimit)

  for (const c of cats) {
    items.push({
      id: `cat:${c.id}`,
      kind: 'jump-category',
      label: c.label,
      description: 'Jump to folder',
      categoryId: c.id,
    })
  }

  return items
}

export function clampActiveIndex(index: number, length: number): number {
  if (length <= 0) return 0
  if (index < 0) return length - 1
  if (index >= length) return 0
  return index
}
