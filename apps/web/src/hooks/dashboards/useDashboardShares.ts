import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { DashboardShare, ShareAccess } from '@/schemas'

export function useDashboardShares(
  dashboardId: string | null
): UseQueryResult<DashboardShare[]> {
  return useQuery({
    queryKey: ['dashboard-shares', dashboardId],
    queryFn: () => api.dashboardShares(dashboardId!),
    enabled: !!dashboardId,
  })
}

type CreateShareVars = { dashboardId: string; email: string; access: ShareAccess }

export function useCreateDashboardShare(): UseMutationResult<DashboardShare, Error, CreateShareVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ dashboardId, email, access }: CreateShareVars) =>
      api.createDashboardShare(dashboardId, { email, access }),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ['dashboard-shares', vars.dashboardId] })
      void qc.invalidateQueries({ queryKey: ['dashboards'] })
      void qc.invalidateQueries({ queryKey: ['dashboard', vars.dashboardId] })
    },
  })
}

type UpdateShareVars = { dashboardId: string; userId: string; access: ShareAccess }

export function useUpdateDashboardShare(): UseMutationResult<DashboardShare, Error, UpdateShareVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ dashboardId, userId, access }: UpdateShareVars) =>
      api.updateDashboardShare(dashboardId, userId, { access }),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ['dashboard-shares', vars.dashboardId] })
      void qc.invalidateQueries({ queryKey: ['dashboards'] })
      void qc.invalidateQueries({ queryKey: ['dashboard', vars.dashboardId] })
    },
  })
}

type DeleteShareVars = { dashboardId: string; userId: string }

export function useDeleteDashboardShare(): UseMutationResult<void, Error, DeleteShareVars> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ dashboardId, userId }: DeleteShareVars) =>
      api.deleteDashboardShare(dashboardId, userId),
    onSuccess: (_data, vars) => {
      void qc.invalidateQueries({ queryKey: ['dashboard-shares', vars.dashboardId] })
      void qc.invalidateQueries({ queryKey: ['dashboards'] })
      void qc.invalidateQueries({ queryKey: ['dashboard', vars.dashboardId] })
    },
  })
}
