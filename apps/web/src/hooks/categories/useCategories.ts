import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '../../api'
import type { Category } from '../../schemas'
import { useUi } from '../../store'

export function useCategories(
  includeHidden?: boolean,
  includeVault?: boolean,
): UseQueryResult<Category[]> {
  const showHidden = useUi((s) => s.showHidden)
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const hidden = includeHidden ?? showHidden
  const vault = includeVault ?? vaultUnlocked
  return useQuery({
    queryKey: ['categories', hidden, vault],
    queryFn: () => api.categories(hidden, vault),
  })
}

type CreateCategoryBody = Partial<Category> & { name: string }

export function useCreateCategory(): UseMutationResult<Category, Error, CreateCategoryBody> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateCategoryBody) => api.createCategory(body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['categories'] })
    },
  })
}

type UpdateCategoryVars = {
  id: string
  body: Partial<{
    name: string
    parent_id: string | null
    is_hidden: boolean
    position: number
  }>
}

export function useUpdateCategory(): UseMutationResult<Category, Error, UpdateCategoryVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }: UpdateCategoryVars) => api.updateCategory(id, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['categories'] })
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
    },
  })
}

export function useDeleteCategory(): UseMutationResult<void, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.deleteCategory(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['categories'] })
      void qc.invalidateQueries({ queryKey: ['bookmarks'] })
    },
  })
}
