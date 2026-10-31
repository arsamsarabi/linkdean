import { useQuery, type UseQueryResult } from '@tanstack/react-query'
import { api } from '../../api'
import type { BootstrapStatus } from '../../schemas'

export function useBootstrap(enabled = true): UseQueryResult<BootstrapStatus> {
  return useQuery({
    queryKey: ['bootstrap'],
    queryFn: api.bootstrap,
    enabled,
  })
}
