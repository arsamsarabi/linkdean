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

export type OrganizationAuditFilters = {
  actorUserId?: string
  action?: string
  from?: string
  to?: string
}

export function useOrganizationAudit(id: string | undefined, filters: OrganizationAuditFilters = {}) {
  const actorUserId = filters.actorUserId?.trim() || undefined
  const action = filters.action?.trim() || undefined
  const from = filters.from?.trim() || undefined
  const to = filters.to?.trim() || undefined
  return useQuery({
    queryKey: ['organizations', id, 'audit', actorUserId, action, from, to],
    queryFn: () =>
      api.listOrganizationAudit(id!, {
        actorUserId,
        action,
        from,
        to,
        limit: 100,
      }),
    enabled: !!id,
  })
}

export async function downloadOrganizationAuditCsv(id: string, filters: OrganizationAuditFilters = {}): Promise<void> {
  const res = await fetch(
    api.organizationAuditCsvUrl(id, {
      actorUserId: filters.actorUserId?.trim() || undefined,
      action: filters.action?.trim() || undefined,
      from: filters.from?.trim() || undefined,
      to: filters.to?.trim() || undefined,
      limit: 1000,
    }),
    { credentials: 'include' }
  )
  if (!res.ok) {
    throw new Error(res.statusText || 'Export failed')
  }
  const blob = await res.blob()
  const a = document.createElement('a')
  const objectUrl = URL.createObjectURL(blob)
  a.href = objectUrl
  a.download = 'organization-audit.csv'
  a.click()
  URL.revokeObjectURL(objectUrl)
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

export function useOrganizationInvites(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id, 'invites'],
    queryFn: () => api.listOrganizationInvites(id!),
    enabled: !!id,
  })
}

export function useCreateOrganizationInvite(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: { email: string; role: 'admin' | 'member' | 'guest'; teams?: { team_id: string }[] }) =>
      api.createOrganizationInvite(id, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'invites'] })
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'members'] })
    },
  })
}

export function useRevokeOrganizationInvite(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (inviteId: string) => api.revokeOrganizationInvite(id, inviteId),
    onSuccess: () => void qc.invalidateQueries({ queryKey: ['organizations', id, 'invites'] }),
  })
}

export function useOrganizationInviteInbox(enabled = true) {
  return useQuery({
    queryKey: ['organization-invites'],
    queryFn: () => api.listOrganizationInviteInbox(),
    enabled,
    refetchInterval: 30_000,
  })
}

export function useAcceptOrganizationInvite() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (inviteId: string) => api.acceptOrganizationInvite(inviteId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organization-invites'] })
      void qc.invalidateQueries({ queryKey: ['organizations'] })
      void qc.invalidateQueries({ queryKey: ['notifications'] })
    },
  })
}

export function useRejectOrganizationInvite() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (inviteId: string) => api.rejectOrganizationInvite(inviteId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organization-invites'] })
      void qc.invalidateQueries({ queryKey: ['notifications'] })
    },
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

export function useOrganizationSso(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id, 'sso'],
    queryFn: () => api.getOrganizationSso(id!),
    enabled: !!id,
  })
}

export function useUpsertOrganizationSso(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: {
      issuer_url: string
      client_id: string
      client_secret?: string
      display_name?: string
      enabled: boolean
    }) => api.upsertOrganizationSso(id, body),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'sso'] })
    },
  })
}

export function useDeleteOrganizationSso(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: () => api.deleteOrganizationSso(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'sso'] })
    },
  })
}

export function useOrganizationScimTokens(id: string | undefined) {
  return useQuery({
    queryKey: ['organizations', id, 'scim-tokens'],
    queryFn: () => api.listOrganizationScimTokens(id!),
    enabled: !!id,
  })
}

export function useCreateOrganizationScimToken(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (name: string) => api.createOrganizationScimToken(id, name),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'scim-tokens'] })
    },
  })
}

export function useRevokeOrganizationScimToken(id: string) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (tokenId: string) => api.revokeOrganizationScimToken(id, tokenId),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: ['organizations', id, 'scim-tokens'] })
    },
  })
}
