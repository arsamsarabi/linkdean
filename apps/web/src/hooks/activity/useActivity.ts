import {
  keepPreviousData,
  useQuery,
  useQueryClient,
  type QueryClient,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { ActivityAction, ActivityEvent, ActivitySubjectType } from '@/schemas'

export type ActivityListParams = {
  subjectType?: ActivitySubjectType | 'all'
  action?: ActivityAction | 'all'
  limit?: number
}

const DEFAULT_LIMIT = 50
/** Warm cache stays usable across reopen / filter hops without a refetch flash. */
const STALE_MS = 30_000

function normalizeParams(params: ActivityListParams = {}): {
  subjectType: ActivitySubjectType | undefined
  action: ActivityAction | undefined
  limit: number
} {
  return {
    subjectType: params.subjectType && params.subjectType !== 'all' ? params.subjectType : undefined,
    action: params.action && params.action !== 'all' ? params.action : undefined,
    limit: params.limit ?? DEFAULT_LIMIT,
  }
}

export function activityQueryKey(params: ActivityListParams = {}): readonly unknown[] {
  const { subjectType, action, limit } = normalizeParams(params)
  return ['activity', 'list', subjectType ?? 'all', action ?? 'all', limit] as const
}

export function prefetchActivity(qc: QueryClient, params: ActivityListParams = {}): Promise<void> {
  const { subjectType, action, limit } = normalizeParams(params)
  return qc.prefetchQuery({
    queryKey: activityQueryKey(params),
    queryFn: () => api.listActivity({ limit, subjectType, action }),
    staleTime: STALE_MS,
  })
}

/** Prefetch default (all/all) history — call on History button hover/focus. */
export function usePrefetchActivity(): () => void {
  const qc = useQueryClient()
  return () => {
    void prefetchActivity(qc)
  }
}

export function useActivity(
  enabled = true,
  params: ActivityListParams = {}
): UseQueryResult<{ events: ActivityEvent[] }> {
  const { subjectType, action, limit } = normalizeParams(params)
  return useQuery({
    queryKey: activityQueryKey(params),
    queryFn: () => api.listActivity({ limit, subjectType, action }),
    enabled,
    retry: 1,
    staleTime: STALE_MS,
    // Filter changes keep prior rows painted until the new page arrives.
    placeholderData: keepPreviousData,
  })
}
