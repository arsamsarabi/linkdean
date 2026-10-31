import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type {
  CreateDashboard,
  Dashboard,
  DashboardSummary,
  PatchDashboard,
  PutDashboardLayout,
} from '@/schemas'

export function useDashboards(): UseQueryResult<DashboardSummary[]> {
  return useQuery({
    queryKey: ['dashboards'] as const,
    queryFn: () => api.listDashboards(),
  })
}

export function useDashboard(id: string | undefined): UseQueryResult<Dashboard> {
  return useQuery({
    queryKey: ['dashboard', id] as const,
    queryFn: () => api.getDashboard(id!),
    enabled: !!id,
  })
}

export function useDefaultDashboard(): UseQueryResult<Dashboard> {
  return useQuery({
    queryKey: ['dashboard', 'default'] as const,
    queryFn: () => api.getDefaultDashboard(),
  })
}

function invalidateDashboards(qc: ReturnType<typeof useQueryClient>): void {
  void qc.invalidateQueries({ queryKey: ['dashboards'] })
  void qc.invalidateQueries({ queryKey: ['dashboard'] })
}

export function useCreateDashboard(): UseMutationResult<Dashboard, Error, CreateDashboard | void> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.createDashboard(body || {}),
    onSuccess: (data) => {
      invalidateDashboards(qc)
      qc.setQueryData(['dashboard', data.id], data)
    },
  })
}

export function usePutDashboard(): UseMutationResult<
  Dashboard,
  Error,
  { id: string; body: PutDashboardLayout }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }) => api.putDashboard(id, body),
    onSuccess: (data) => {
      invalidateDashboards(qc)
      qc.setQueryData(['dashboard', data.id], data)
      if (data.is_default) qc.setQueryData(['dashboard', 'default'], data)
    },
  })
}

export function usePutDefaultDashboard(): UseMutationResult<Dashboard, Error, PutDashboardLayout> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body) => api.putDefaultDashboard(body),
    onSuccess: (data) => {
      invalidateDashboards(qc)
      qc.setQueryData(['dashboard', 'default'], data)
      qc.setQueryData(['dashboard', data.id], data)
    },
  })
}

export function usePatchDashboard(): UseMutationResult<
  Dashboard,
  Error,
  { id: string; body: PatchDashboard }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, body }) => api.patchDashboard(id, body),
    onSuccess: (data) => {
      invalidateDashboards(qc)
      qc.setQueryData(['dashboard', data.id], data)
      if (data.is_default) qc.setQueryData(['dashboard', 'default'], data)
    },
  })
}

export function useDeleteDashboard(): UseMutationResult<DashboardSummary[], Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id) => api.deleteDashboard(id),
    onSuccess: () => {
      invalidateDashboards(qc)
    },
  })
}
