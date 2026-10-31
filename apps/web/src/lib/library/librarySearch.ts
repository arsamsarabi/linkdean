export const SORT_KEYS = ['updated_desc', 'updated_asc', 'title_asc', 'title_desc', 'created_desc'] as const

export type SortKey = (typeof SORT_KEYS)[number]

export const FLAG_KEYS = ['unread', 'favorite', 'pinned'] as const
export type FlagKey = (typeof FLAG_KEYS)[number]

export type LibrarySearch = {
  q?: string
  sort?: SortKey
  /** One or many category ids */
  category?: string | string[]
  /** One or many tags */
  tag?: string | string[]
  /** Reading-list / favorite / pin filters */
  flag?: string | string[]
  /** Explorer folder id (`uncategorized` or category uuid). Omit / `all` = root. */
  folder?: string
  /** One-shot: open create drawer (command palette). Consumed then stripped. */
  create?: true
  /** One-shot: focus library search input. Consumed then stripped. */
  focus?: 'search'
}

export function asStringList(v: unknown): string[] {
  if (v == null || v === '') return []
  if (Array.isArray(v)) return v.filter((x): x is string => typeof x === 'string' && x.length > 0)
  if (typeof v === 'string') {
    return v
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean)
  }
  return []
}

export function listToSearchParam(xs: string[]): string | string[] | undefined {
  if (xs.length === 0) return undefined
  if (xs.length === 1) return xs[0]
  return xs
}

export function validateLibrarySearch(search: Record<string, unknown>): LibrarySearch {
  const sortRaw = typeof search.sort === 'string' ? search.sort : undefined
  const sort = sortRaw && (SORT_KEYS as readonly string[]).includes(sortRaw) ? (sortRaw as SortKey) : undefined

  const category = asStringList(search.category)
  const tag = asStringList(search.tag)
  const flag = asStringList(search.flag).filter((f): f is FlagKey =>
    (FLAG_KEYS as readonly string[]).includes(f)
  )
  const q = typeof search.q === 'string' && search.q.trim() ? search.q : undefined
  const folderRaw = typeof search.folder === 'string' ? search.folder.trim() : ''
  const folder = folderRaw && folderRaw !== 'all' ? folderRaw : undefined
  const createRaw = search.create
  const create =
    createRaw === true || createRaw === 1 || createRaw === '1' || createRaw === 'true' ? true : undefined
  const focus = search.focus === 'search' ? ('search' as const) : undefined

  return {
    q,
    sort: sort && sort !== 'updated_desc' ? sort : undefined,
    category: listToSearchParam(category),
    tag: listToSearchParam(tag),
    flag: listToSearchParam(flag),
    folder,
    create,
    focus,
  }
}

/** Build next search object; omit empty/default keys. */
export function buildLibrarySearch(input: {
  q?: string
  sort?: SortKey
  category?: string[]
  tag?: string[]
  flag?: string[]
  folder?: string
  create?: boolean
  focus?: 'search'
}): LibrarySearch {
  const q = input.q?.trim() || undefined
  const sort = input.sort && input.sort !== 'updated_desc' ? input.sort : undefined
  const folderRaw = input.folder?.trim() || undefined
  const folder = folderRaw && folderRaw !== 'all' ? folderRaw : undefined
  const flag = (input.flag || []).filter((f): f is FlagKey =>
    (FLAG_KEYS as readonly string[]).includes(f)
  )
  return {
    q,
    sort,
    category: listToSearchParam(input.category || []),
    tag: listToSearchParam(input.tag || []),
    flag: listToSearchParam(flag),
    folder,
    create: input.create ? true : undefined,
    focus: input.focus === 'search' ? 'search' : undefined,
  }
}
