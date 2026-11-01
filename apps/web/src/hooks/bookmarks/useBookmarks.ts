import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { Bookmark, SearchResponse } from '@/schemas'
import { useUi } from '@/stores'

export function useBookmarks(categoryId?: string | null, includeVault?: boolean): UseQueryResult<Bookmark[]> {
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const vault = includeVault ?? vaultUnlocked
  return useQuery({
    queryKey:
      categoryId != null && categoryId !== ''
        ? (['bookmarks', vault, categoryId] as const)
        : (['bookmarks', vault] as const),
    queryFn: () => api.bookmarks(categoryId ?? null, vault),
  })
}

export function useSearchBookmarks(q: string, enabled: boolean): UseQueryResult<SearchResponse> {
  return useQuery({
    queryKey: ['search', q],
    queryFn: () => api.search(q),
    enabled,
  })
}

type CreateBookmarkBody = {
  url: string
  vault_id?: string | null
  title?: string
  description?: string
  notes?: string
  category_id?: string | null
  category_ids?: string[]
  tags: string[]
  is_favorite?: boolean
  is_unread?: boolean
  is_pinned?: boolean
}

export function useCreateBookmark(): UseMutationResult<Bookmark, Error, CreateBookmarkBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateBookmarkBody) => api.createBookmark(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
      void qc.invalidateQueries({ queryKey: ['search'] })
      window.setTimeout(() => void qc.invalidateQueries({ queryKey: ['bookmarks'] }), 2500)
      window.setTimeout(() => void qc.invalidateQueries({ queryKey: ['bookmarks'] }), 6000)
    },
  })
}

type UpdateBookmarkVars = {
  id: string
  body: Partial<{
    url: string
    title: string
    description: string | null
    notes: string | null
    category_ids: string[]
    tags: string[]
    is_favorite: boolean
    is_unread: boolean
    is_pinned: boolean
  }>
}

export function useUpdateBookmark(): UseMutationResult<Bookmark, Error, UpdateBookmarkVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: UpdateBookmarkVars) => api.updateBookmark(id, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
      void qc.invalidateQueries({ queryKey: ['search'] })
      window.setTimeout(() => void qc.invalidateQueries({ queryKey: ['bookmarks'] }), 2500)
      window.setTimeout(() => void qc.invalidateQueries({ queryKey: ['bookmarks'] }), 6000)
    },
  })
}

export function useDeleteBookmark(): UseMutationResult<void, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.deleteBookmark(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
      void qc.invalidateQueries({ queryKey: ['search'] })
      void qc.invalidateQueries({ queryKey: ['trash'] })
    },
  })
}

type PreviewVars = { id: string; blob: Blob }

export function useUploadBookmarkPreview(): UseMutationResult<Bookmark, Error, PreviewVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, blob }: PreviewVars) => api.uploadBookmarkPreview(id, blob),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
      void qc.invalidateQueries({ queryKey: ['search'] })
      void qc.invalidateQueries({ queryKey: ['dashboards'] })
    },
  })
}

export function useDeleteBookmarkPreview(): UseMutationResult<Bookmark, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.deleteBookmarkPreview(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
      void qc.invalidateQueries({ queryKey: ['search'] })
      void qc.invalidateQueries({ queryKey: ['dashboards'] })
    },
  })
}

export function useSaveBookmarkCopy(): UseMutationResult<Bookmark, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.saveBookmarkCopy(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
      void qc.invalidateQueries({ queryKey: ['search'] })
      window.setTimeout(() => void qc.invalidateQueries({ queryKey: ['bookmarks'] }), 2500)
      window.setTimeout(() => void qc.invalidateQueries({ queryKey: ['bookmarks'] }), 6000)
    },
  })
}
