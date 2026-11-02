export const SORT_KEYS = ['updated_desc', 'updated_asc', 'title_asc', 'title_desc', 'created_desc'] as const

export type SortKey = (typeof SORT_KEYS)[number]

export const FLAG_KEYS = ['unread', 'favorite', 'pinned'] as const
export type FlagKey = (typeof FLAG_KEYS)[number]

export const SINCE_KEYS = ['7d', '30d', '90d', 'year'] as const
export type SinceKey = (typeof SINCE_KEYS)[number]

export type LibrarySearch = {
  q?: string
  sort?: SortKey
  /** One or many category ids */
  category?: string | string[]
  /** One or many tags */
  tag?: string | string[]
  /** Reading-list / favorite / pin filters */
  flag?: string | string[]
  /** Created within window: 7d / 30d / 90d / year */
  since?: SinceKey
  /** Vault-only filter (`1` in URL). */
  vault?: '1'
  /** Vaults page: limit to one open vault. */
  vaultId?: string
  /** Explorer folder id (`uncategorized` or category uuid). Omit / `all` = root. */
  folder?: string
  /** One-shot: open create drawer (command palette). Consumed then stripped. */
  create?: true
  /** One-shot: focus library search input. Consumed then stripped. */
  focus?: 'search'
  /** One-shot: open the create drawer with this URL filled in (paste to add). */
  addUrl?: string
}

/** A pasted or shared string that is a single web address. */
export function asWebUrl(text: string | null | undefined): string | undefined {
  const t = text?.trim()
  if (!t || /\s/.test(t) || !/^https?:\/\//i.test(t)) return undefined
  try {
    new URL(t)
    return t
  } catch {
    return undefined
  }
}

// Known issue: A tag containing a comma cannot be filtered, since lists are comma-joined in
// the URL. Fix: repeat the param instead.
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
  const flag = asStringList(search.flag).filter((f): f is FlagKey => (FLAG_KEYS as readonly string[]).includes(f))
  const sinceRaw = typeof search.since === 'string' ? search.since : undefined
  const since = sinceRaw && (SINCE_KEYS as readonly string[]).includes(sinceRaw) ? (sinceRaw as SinceKey) : undefined
  const vaultRaw = search.vault
  const vault =
    vaultRaw === true || vaultRaw === 1 || vaultRaw === '1' || vaultRaw === 'true' ? ('1' as const) : undefined
  const vaultId = typeof search.vaultId === 'string' && search.vaultId ? search.vaultId : undefined
  const q = typeof search.q === 'string' && search.q.trim() ? search.q : undefined
  const folderRaw = typeof search.folder === 'string' ? search.folder.trim() : ''
  const folder = folderRaw && folderRaw !== 'all' ? folderRaw : undefined
  const createRaw = search.create
  const create = createRaw === true || createRaw === 1 || createRaw === '1' || createRaw === 'true' ? true : undefined
  const focus = search.focus === 'search' ? ('search' as const) : undefined
  const addUrl = typeof search.addUrl === 'string' ? asWebUrl(search.addUrl) : undefined

  return {
    q,
    sort: sort && sort !== 'updated_desc' ? sort : undefined,
    category: listToSearchParam(category),
    tag: listToSearchParam(tag),
    flag: listToSearchParam(flag),
    since,
    vault,
    vaultId,
    folder,
    create,
    focus,
    addUrl,
  }
}

/** Build next search object; omit empty/default keys. */
export function buildLibrarySearch(input: {
  q?: string
  sort?: SortKey
  category?: string[]
  tag?: string[]
  flag?: string[]
  since?: SinceKey | null
  vault?: boolean
  vaultId?: string | null
  folder?: string
  create?: boolean
  focus?: 'search'
  addUrl?: string
}): LibrarySearch {
  const q = input.q?.trim() || undefined
  const sort = input.sort && input.sort !== 'updated_desc' ? input.sort : undefined
  const folderRaw = input.folder?.trim() || undefined
  const folder = folderRaw && folderRaw !== 'all' ? folderRaw : undefined
  const flag = (input.flag || []).filter((f): f is FlagKey => (FLAG_KEYS as readonly string[]).includes(f))
  const since = input.since && (SINCE_KEYS as readonly string[]).includes(input.since) ? input.since : undefined
  return {
    q,
    sort,
    category: listToSearchParam(input.category || []),
    tag: listToSearchParam(input.tag || []),
    flag: listToSearchParam(flag),
    since,
    vault: input.vault ? '1' : undefined,
    vaultId: input.vaultId || undefined,
    folder,
    create: input.create ? true : undefined,
    focus: input.focus === 'search' ? 'search' : undefined,
    addUrl: asWebUrl(input.addUrl),
  }
}
