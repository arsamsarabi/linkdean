import { useMutation, useQueryClient, type UseMutationResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { Bookmark } from '@/schemas'
import { unionIds, unionTags } from '@/services'

export type BulkAddCategoriesVars = {
  ids: string[]
  category_ids: string[]
  /** Needed so we can union without an extra fetch. */
  byId: Map<string, Bookmark>
}

export type BulkTagVars = {
  ids: string[]
  tags: string[]
  mode: 'add' | 'replace'
  /** Needed for add-mode so we can union without an extra fetch. */
  byId: Map<string, Bookmark>
}

export type BulkDeleteVars = {
  ids: string[]
}

export type BulkResult = {
  ok: number
  failed: number
  errors: string[]
}

async function runAll(ids: string[], fn: (id: string) => Promise<unknown>): Promise<BulkResult> {
  let ok = 0
  const errors: string[] = []
  for (const id of ids) {
    try {
      await fn(id)
      ok += 1
    } catch (e) {
      errors.push(`${id}: ${String(e)}`)
    }
  }
  return { ok, failed: errors.length, errors }
}

function invalidate(qc: ReturnType<typeof useQueryClient>): void {
  void qc.invalidateQueries({ queryKey: ['bookmarks'] })
  void qc.invalidateQueries({ queryKey: ['search'] })
  void qc.invalidateQueries({ queryKey: ['duplicates'] })
  void qc.invalidateQueries({ queryKey: ['trash'] })
  window.setTimeout(() => void qc.invalidateQueries({ queryKey: ['bookmarks'] }), 2500)
  window.setTimeout(() => void qc.invalidateQueries({ queryKey: ['bookmarks'] }), 6000)
}

export function useBulkAddCategories(): UseMutationResult<BulkResult, Error, BulkAddCategoriesVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ ids, category_ids, byId }: BulkAddCategoriesVars) =>
      runAll(ids, (id) => {
        const existing = (byId.get(id)?.categories || []).map((c) => c.id)
        const next = unionIds(existing, category_ids)
        return api.updateBookmark(id, { category_ids: next })
      }),
    onSuccess: () => invalidate(qc),
  })
}

export function useBulkTagBookmarks(): UseMutationResult<BulkResult, Error, BulkTagVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ ids, tags, mode, byId }: BulkTagVars) =>
      runAll(ids, (id) => {
        const next =
          mode === 'replace' ? tags.map((t) => t.trim()).filter(Boolean) : unionTags(byId.get(id)?.tags ?? [], tags)
        return api.updateBookmark(id, { tags: next })
      }),
    onSuccess: () => invalidate(qc),
  })
}

export function useBulkDeleteBookmarks(): UseMutationResult<BulkResult, Error, BulkDeleteVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ ids }: BulkDeleteVars) => runAll(ids, (id) => api.deleteBookmark(id)),
    onSuccess: () => invalidate(qc),
  })
}
