import { useMutation, useQueryClient, type UseMutationResult } from '@tanstack/react-query'
import { api } from '@/api'

// Adding happens on the server, against what each bookmark holds when it saves. A list
// snapshot from the browser could be stale, or miss a bookmark and wipe its tags.
export type BulkAddCategoriesVars = {
  ids: string[]
  category_ids: string[]
}

export type BulkTagVars = {
  ids: string[]
  tags: string[]
  mode: 'add' | 'replace'
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
    mutationFn: ({ ids, category_ids }: BulkAddCategoriesVars) =>
      runAll(ids, (id) => api.updateBookmark(id, { add_category_ids: category_ids })),
    onSuccess: () => invalidate(qc),
  })
}

export function useBulkTagBookmarks(): UseMutationResult<BulkResult, Error, BulkTagVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ ids, tags, mode }: BulkTagVars) => {
      const clean = tags.map((t) => t.trim()).filter(Boolean)
      return runAll(ids, (id) => api.updateBookmark(id, mode === 'replace' ? { tags: clean } : { add_tags: clean }))
    },
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
