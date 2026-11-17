import {
  useMutation,
  useQuery,
  useQueryClient,
  type QueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import { archivePending } from '@/lib'
import type { ArchiveFormat, Bookmark, BookmarkArchive, BulkArchiveResult } from '@/schemas'

/** How often a bookmark's copies are checked while one is queued or being captured. */
export const ARCHIVE_POLL_MS = 3000

export function archivesQueryKey(id: string): readonly unknown[] {
  return ['archives', id]
}

/** Every copy of a bookmark, polled while one is on its way. */
export function useArchives(id: string | undefined, enabled = true): UseQueryResult<BookmarkArchive[]> {
  return useQuery({
    queryKey: archivesQueryKey(id ?? ''),
    queryFn: () => api.listArchives(id!),
    enabled: enabled && !!id,
    refetchInterval: (q) => (archivePending(q.state.data) ? ARCHIVE_POLL_MS : false),
  })
}

/** One bookmark, for the archive viewer's header. */
export function useBookmark(id: string): UseQueryResult<Bookmark> {
  return useQuery({ queryKey: ['bookmarks', 'one', id], queryFn: () => api.bookmark(id) })
}

function refreshLists(qc: QueryClient): void {
  void qc.invalidateQueries({ queryKey: ['bookmarks'] })
  void qc.invalidateQueries({ queryKey: ['quota-usage'] })
}

export function useRequestArchive(): UseMutationResult<
  BookmarkArchive[],
  Error,
  { id: string; formats: ArchiveFormat[] }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, formats }) => api.requestArchive(id, formats),
    onSuccess: (archives, { id }) => {
      qc.setQueryData(archivesQueryKey(id), archives)
      refreshLists(qc)
      // Cards learn of the finished copy from the list; a capture takes a few seconds.
      window.setTimeout(() => refreshLists(qc), 10000)
      window.setTimeout(() => refreshLists(qc), 30000)
    },
  })
}

export function useBulkArchive(): UseMutationResult<
  BulkArchiveResult,
  Error,
  { ids: string[]; formats: ArchiveFormat[] }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ ids, formats }) => api.bulkArchive(ids, formats),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['archives'] })
      refreshLists(qc)
    },
  })
}

export function useDeleteArchives(): UseMutationResult<{ ok: boolean }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id) => api.deleteArchives(id),
    onSuccess: (_, id) => {
      qc.setQueryData(archivesQueryKey(id), [])
      refreshLists(qc)
    },
  })
}
