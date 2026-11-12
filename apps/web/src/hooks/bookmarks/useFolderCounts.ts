import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { FolderCounts } from '@/schemas'

/** Bookmarks per folder for Explorer's tiles. Under `bookmarks`, so any bookmark edit refreshes it. */
export function useFolderCounts(enabled = true): UseQueryResult<FolderCounts> {
  return useQuery({
    queryKey: ['bookmarks', 'folder-counts'],
    queryFn: () => api.folderCounts(),
    enabled,
  })
}
