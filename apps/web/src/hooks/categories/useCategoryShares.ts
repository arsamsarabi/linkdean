import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type {
  CategoryShare,
  CategorySharesEnvelope,
  CreateCategoryShareOutcome,
  PendingShareInvite,
  ShareAccess,
} from '@/schemas'

export function useCategoryShares(categoryId: string | null): UseQueryResult<CategorySharesEnvelope> {
  return useQuery({
    queryKey: ['category-shares', categoryId],
    queryFn: () => api.categoryShares(categoryId!),
    enabled: !!categoryId,
  })
}

type CreateShareVars = { categoryId: string; email: string; access: ShareAccess }

export function useCreateCategoryShare(): UseMutationResult<CreateCategoryShareOutcome, Error, CreateShareVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ categoryId, email, access }: CreateShareVars) =>
      api.createCategoryShare(categoryId, { email, access }),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ['category-shares', vars.categoryId] })
      void qc.invalidateQueries({ queryKey: ['categories'] })
    },
  })
}

type UpdateShareVars = { categoryId: string; userId: string; access: ShareAccess }

export function useUpdateCategoryShare(): UseMutationResult<CategoryShare, Error, UpdateShareVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ categoryId, userId, access }: UpdateShareVars) =>
      api.updateCategoryShare(categoryId, userId, { access }),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ['category-shares', vars.categoryId] })
      void qc.invalidateQueries({ queryKey: ['categories'] })
    },
  })
}

type DeleteShareVars = { categoryId: string; userId: string }

export function useDeleteCategoryShare(): UseMutationResult<void, Error, DeleteShareVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ categoryId, userId }: DeleteShareVars) => api.deleteCategoryShare(categoryId, userId),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ['category-shares', vars.categoryId] })
      void qc.invalidateQueries({ queryKey: ['categories'] })
    },
  })
}

type UpdatePendingVars = { categoryId: string; pendingId: string; access: ShareAccess }

export function useUpdateCategoryPendingShare(): UseMutationResult<PendingShareInvite, Error, UpdatePendingVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ categoryId, pendingId, access }: UpdatePendingVars) =>
      api.updateCategoryPendingShare(categoryId, pendingId, { access }),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ['category-shares', vars.categoryId] })
    },
  })
}

type DeletePendingVars = { categoryId: string; pendingId: string }

export function useDeleteCategoryPendingShare(): UseMutationResult<void, Error, DeletePendingVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ categoryId, pendingId }: DeletePendingVars) => api.deleteCategoryPendingShare(categoryId, pendingId),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ['category-shares', vars.categoryId] })
    },
  })
}
