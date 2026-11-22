import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'

export function useOrganizations(enabled = true) {
  return useQuery({
    queryKey: ['organizations'],
    queryFn: api.listOrganizations,
    enabled,
  })
}

export function useOrganization(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id],
    queryFn: () => api.getOrganization(id!),
    enabled: !!id,
  })
}

export function useOrganizationMembers(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id, 'members'],
    queryFn: () => api.listOrganizationMembers(id!),
    enabled: !!id,
  })
}

export function useOrganizationTeams(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id, 'teams'],
    queryFn: () => api.listOrganizationTeams(id!),
    enabled: !!id,
  })
}

export function useOrganizationAudit(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id, 'audit'],
    queryFn: () => api.listOrganizationAudit(id!, { limit: 100 }),
    enabled: !!id,
  })
}

function invalidateOrg(qc: ReturnType<typeof useQueryClient>, id?: string) {
  void qc.invalidateQueries({ queryKey: ['organizations'] })
  if (id) void qc.invalidateQueries({ queryKey: ['organizations', id] })
}

export function useCreateOrganization() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (name: string) => api.createOrganization({ name }),
    onSuccess: () => invalidateOrg(qc),
  })
}

export function useDeleteOrganization(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (confirmName: string) => api.deleteOrganization(id, confirmName),
    onSuccess: () => invalidateOrg(qc, id),
  })
}

export function useRestoreOrganization(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.restoreOrganization(id),
    onSuccess: () => invalidateOrg(qc, id),
  })
}

export function useArchiveTeam(orgId: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (teamId: string) => api.archiveOrganizationTeam(orgId, teamId),
    onSuccess: () => invalidateOrg(qc, orgId),
  })
}

export function useUnarchiveTeam(orgId: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (teamId: string) => api.unarchiveOrganizationTeam(orgId, teamId),
    onSuccess: () => invalidateOrg(qc, orgId),
  })
}

export function useDeleteTeam(orgId: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (teamId: string) => api.deleteOrganizationTeam(orgId, teamId),
    onSuccess: () => invalidateOrg(qc, orgId),
  })
}
