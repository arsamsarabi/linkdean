import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { AbuseReport, NewAbuseReport, ReportConfig, ResolveAbuseReport } from '@/schemas'

/** Where to send abuse mail that does not fit the form. Loads only while the form is open. */
export function useReportConfig(enabled = true): UseQueryResult<ReportConfig> {
  return useQuery({ queryKey: ['reports', 'config'], queryFn: () => api.reportConfig(), enabled })
}

export function useCreateReport(): UseMutationResult<{ ok: boolean; id: string }, Error, NewAbuseReport> {
  return useMutation({ mutationFn: (body) => api.createReport(body) })
}

export function useAbuseReports(
  status: 'open' | 'resolved',
  enabled = true
): UseQueryResult<{ reports: AbuseReport[] }> {
  return useQuery({ queryKey: ['admin', 'reports', status], queryFn: () => api.listAbuseReports(status), enabled })
}

export function useResolveAbuseReport(): UseMutationResult<
  { report: AbuseReport },
  Error,
  { id: string } & ResolveAbuseReport
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, ...body }) => api.resolveAbuseReport(id, body),
    // A ban also changes the banned list.
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['admin'] }),
  })
}
