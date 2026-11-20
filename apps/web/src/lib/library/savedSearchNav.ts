import {
  buildLibrarySearch,
  type FlagKey,
  type LibrarySearch,
  type SinceKey,
  type SortKey,
} from '@/lib/library/librarySearch'
import type { SavedSearchFilters } from '@/schemas'

const SORT = new Set(['updated_desc', 'updated_asc', 'title_asc', 'title_desc', 'created_desc'])
const FLAGS = new Set(['unread', 'favorite', 'pinned'])
const SINCE = new Set(['7d', '30d', '90d', 'year'])

/** Turn a saved filter snapshot into library URL search params. */
export function librarySearchFromSavedFilters(filters: SavedSearchFilters): LibrarySearch {
  const sort =
    filters.sort && SORT.has(filters.sort) && filters.sort !== 'updated_desc' ? (filters.sort as SortKey) : undefined
  const flag = (filters.flag ?? []).filter((f): f is FlagKey => FLAGS.has(f))
  const since = filters.since && SINCE.has(filters.since) ? (filters.since as SinceKey) : undefined
  return buildLibrarySearch({
    q: filters.q,
    sort,
    category: filters.category ?? [],
    tag: filters.tag ?? [],
    flag,
    since: since ?? null,
    vault: !!filters.vault,
    vaultId: filters.vault_id ?? null,
    folder: filters.folder,
  })
}

/** Path to open for a saved search (vault-oriented filters use /vaults). */
export function pathForSavedFilters(filters: SavedSearchFilters): '/library' | '/vaults' {
  return filters.vault || filters.vault_id ? '/vaults' : '/library'
}

/** Snapshot current library chrome into API filters (no ephemeral keys). */
export function filtersFromLibraryState(input: {
  q: string
  sort: SortKey
  category: string[]
  tag: string[]
  flag: string[]
  since: SinceKey | null
  vault: boolean
  vaultId: string | null
  folder: string
}): SavedSearchFilters {
  const flag = input.flag.filter((f): f is FlagKey => FLAGS.has(f))
  const folder = input.folder && input.folder !== 'all' ? input.folder : undefined
  return {
    q: input.q.trim() || undefined,
    sort: input.sort !== 'updated_desc' ? input.sort : undefined,
    category: input.category,
    tag: input.tag,
    flag,
    since: input.since ?? undefined,
    vault: input.vault || undefined,
    vault_id: input.vaultId || undefined,
    folder,
  }
}
