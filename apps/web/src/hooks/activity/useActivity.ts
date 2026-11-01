import { keepPreviousData, useQuery, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { ActivityAction, ActivityEvent, ActivitySubjectType } from '@/schemas'

export type ActivityListParams = {
  subjectType?: ActivitySubjectType | 'all'
  action?: ActivityAction | 'all'
  limit?: number
}

export function useActivity(
  enabled = true,
  params: ActivityListParams = {}
): UseQueryResult<{ events: ActivityEvent[] }> {
  const subjectType = params.subjectType && params.subjectType !== 'all' ? params.subjectType : undefined
  const action = params.action && params.action !== 'all' ? params.action : undefined
  const limit = params.limit ?? 50
  return useQuery({
    queryKey: ['activity', 'list', subjectType ?? 'all', action ?? 'all', limit],
    queryFn: () => api.listActivity({ limit, subjectType, action }),
    enabled,
    retry: 1,
    placeholderData: keepPreviousData,
  })
}
