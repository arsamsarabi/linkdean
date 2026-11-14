import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { ExperimentalFlags } from '@/schemas'

const OFF: ExperimentalFlags = { agent_skills: false }

/** Experimental features the server has turned on. Every flag reads off until the answer arrives. */
export function useExperimentalFlags(): ExperimentalFlags {
  const { data } = useQuery({
    queryKey: ['auth-config'],
    queryFn: api.authConfig,
    staleTime: Infinity,
  })
  return data?.experimental ?? OFF
}
