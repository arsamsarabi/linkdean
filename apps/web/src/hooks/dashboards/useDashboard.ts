import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { Dashboard, PutDashboardLayout } from '@/schemas'

export function useDefaultDashboard(): UseQueryResult<Dashboard> {
  return useQuery({
    queryKey: ['dashboard', 'default'] as const,
    queryFn: () => api.getDefaultDashboard(),
  })
}

export function usePutDefaultDashboard(): UseMutationResult<Dashboard, Error, PutDashboardLayout> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.putDefaultDashboard(body),
    onSuccess: (data) => {
      qc.setQueryData(['dashboard', 'default'], data)
    },
  })
}
