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

export function useOrganizationInviteLinks(id: string | undefined, enabled = true) {
  return useQuery({
    queryKey: ['organizations', id, 'invite-links'],
    queryFn: () => api.listOrganizationInviteLinks(id!),
    enabled: !!id && enabled,
  })
}

export function usePatchOrganizationInviteLinks(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (invite_links_enabled: boolean) =>
      api.patchOrganizationInviteLinksSettings(id, { invite_links_enabled }),
    onSuccess: () => invalidateOrg(qc, id),
  })
}

export function useCreateOrganizationInviteLink(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: {
      role: 'admin' | 'member' | 'guest'
      expires_in_days?: number
      max_uses?: number | null
      teams?: { team_id: string; role?: 'maintainer' | 'editor' | 'viewer' }[]
    }) => api.createOrganizationInviteLink(id, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'invite-links'] })
    },
  })
}

export function useRevokeOrganizationInviteLink(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (linkId: string) => api.revokeOrganizationInviteLink(id, linkId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'invite-links'] })
    },
  })
}

export function useAcceptOrganizationInviteLink() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (code: string) => api.acceptOrganizationInviteLink(code),
    onSuccess: () => invalidateOrg(qc),
  })
}

export function useOrganizationDomains(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id, 'domains'],
    queryFn: () => api.listOrganizationDomains(id!),
    enabled: !!id,
  })
}

export function useCreateOrganizationDomain(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (domain: string) => api.createOrganizationDomain(id, domain),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'domains'] })
    },
  })
}

export function useDeleteOrganizationDomain(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (domainId: string) => api.deleteOrganizationDomain(id, domainId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'domains'] })
    },
  })
}

export function useVerifyOrganizationDomain(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (domainId: string) => api.verifyOrganizationDomain(id, domainId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'domains'] })
    },
  })
}

export function usePatchOrganizationDomain(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (args: { domainId: string; join_mode: 'auto_join' | 'request' }) =>
      api.patchOrganizationDomain(id, args.domainId, args.join_mode),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'domains'] })
    },
  })
}

export function useOrganizationDomainJoinRequests(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id, 'domain-join-requests'],
    queryFn: () => api.listOrganizationDomainJoinRequests(id!),
    enabled: !!id,
  })
}

export function useApproveDomainJoinRequest(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (requestId: string) => api.approveOrganizationDomainJoinRequest(id, requestId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'domain-join-requests'] })
      invalidateOrg(qc, id)
    },
  })
}

export function useDenyDomainJoinRequest(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (requestId: string) => api.denyOrganizationDomainJoinRequest(id, requestId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'domain-join-requests'] })
    },
  })
}
