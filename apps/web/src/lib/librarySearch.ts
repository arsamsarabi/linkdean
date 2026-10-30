export const SORT_KEYS = [
  'updated_desc',
  'updated_asc',
  'title_asc',
  'title_desc',
  'created_desc',
] as const

export type SortKey = (typeof SORT_KEYS)[number]

export type LibrarySearch = {
  q?: string
  sort?: SortKey
  /** One or many category ids */
  category?: string | string[]
  /** One or many tags */
  tag?: string | string[]
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
  const sort =
    sortRaw && (SORT_KEYS as readonly string[]).includes(sortRaw)
      ? (sortRaw as SortKey)
      : undefined

  const category = asStringList(search.category)
  const tag = asStringList(search.tag)
  const q = typeof search.q === 'string' && search.q.trim() ? search.q : undefined

  return {
    q,
    sort: sort && sort !== 'updated_desc' ? sort : undefined,
    category: listToSearchParam(category),
    tag: listToSearchParam(tag),
  }
}

/** Build next search object; omit empty/default keys. */
export function buildLibrarySearch(input: {
  q?: string
  sort?: SortKey
  category?: string[]
  tag?: string[]
}): LibrarySearch {
  const q = input.q?.trim() || undefined
  const sort = input.sort && input.sort !== 'updated_desc' ? input.sort : undefined
  return {
    q,
    sort,
    category: listToSearchParam(input.category || []),
    tag: listToSearchParam(input.tag || []),
  }
}
