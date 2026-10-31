import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { api } from '../../api'
import type { MeResponse } from '../../schemas'

export function useMe(enabled = true): UseQueryResult<MeResponse> {
  return useQuery({
    queryKey: ['me'],
    queryFn: api.me,
    enabled,
  })
}
