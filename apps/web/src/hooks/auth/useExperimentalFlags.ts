import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import type { ExperimentalFlags } from '@/schemas'

const OFF: ExperimentalFlags = { agent_skills: false }

function useAuthConfigQuery() {
  return useQuery({
    queryKey: ['auth-config'],
    queryFn: api.authConfig,
    staleTime: Infinity,
  })
}

/** Experimental features the server has turned on. Every flag reads off until the answer arrives. */
export function useExperimentalFlags(): ExperimentalFlags {
  const { data } = useAuthConfigQuery()
  return data?.experimental ?? OFF
}

/** The running server's version, such as `0.0.12`, once `/auth/config` has answered. */
export function useAppVersion(): string | undefined {
  const { data } = useAuthConfigQuery()
  return data?.version ?? undefined
}

/**
 * Whether the operator allows archiving. Defaults to true until `/auth/config` answers
 * (matches the platform default); then follows the server. Existing Open/Download stay available.
 */
export function useArchivesEnabled(): boolean {
  const { data } = useAuthConfigQuery()
  return data?.archives_enabled ?? true
}
