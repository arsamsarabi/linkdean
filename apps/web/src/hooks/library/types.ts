import type { SinceKey, SortKey } from '@/lib'
import type { FlagFilter, SinceFilter } from '@/services'

export type LibraryViewMode = 'grid' | 'explorer'

export type LibrarySearchParams = {
  q?: string
  sort?: SortKey
  category?: string | string[]
  tag?: string | string[]
  flag?: string | string[]
  since?: SinceKey
  vault?: '1'
  folder?: string
  create?: true
  focus?: 'search'
}

export type LibrarySearchPatch = {
  q?: string
  sort?: SortKey
  category?: string[]
  tag?: string[]
  flag?: string[]
  since?: SinceFilter | null
  vault?: boolean
  /** Pass `all` or empty string to clear folder selection. */
  folder?: string
}

export type { FlagFilter, SinceFilter }
