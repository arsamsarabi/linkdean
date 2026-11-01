import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { CategoryPublicLink } from '@/schemas'

export function useCategoryPublicLink(categoryId: string | null, enabled = true): UseQueryResult<CategoryPublicLink> {
  return useQuery({
    queryKey: ['category-public-link', categoryId],
    queryFn: () => api.categoryPublicLink(categoryId!),
    enabled: !!categoryId && enabled,
  })
}

export function useEnableCategoryPublicLink(): UseMutationResult<CategoryPublicLink, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (categoryId: string) => api.enableCategoryPublicLink(categoryId),
    onSuccess: (_data, categoryId) => {
      void qc.invalidateQueries({ queryKey: ['category-public-link', categoryId] })
    },
  })
}

export function useRotateCategoryPublicLink(): UseMutationResult<CategoryPublicLink, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (categoryId: string) => api.rotateCategoryPublicLink(categoryId),
    onSuccess: (_data, categoryId) => {
      void qc.invalidateQueries({ queryKey: ['category-public-link', categoryId] })
    },
  })
}

export function useDisableCategoryPublicLink(): UseMutationResult<CategoryPublicLink, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (categoryId: string) => api.disableCategoryPublicLink(categoryId),
    onSuccess: (_data, categoryId) => {
      void qc.invalidateQueries({ queryKey: ['category-public-link', categoryId] })
    },
  })
}
