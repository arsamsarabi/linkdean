import { z } from 'zod'

export const organizationRoleSchema = z.enum(['owner', 'admin', 'member', 'guest'])
export const teamRoleSchema = z.enum(['maintainer', 'editor', 'viewer'])

export const organizationWorkspaceSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  role: organizationRoleSchema,
  deleted_at: z.string().nullish(),
  invite_links_enabled: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
})

export const organizationInviteLinkViewSchema = z.object({
  id: z.string().uuid(),
  organization_id: z.string().uuid(),
  organization_name: z.string(),
  code_prefix: z.string(),
  role: organizationRoleSchema,
  created_by_user_id: z.string().uuid(),
  expires_at: z.string(),
  max_uses: z.number().int().nullable(),
  use_count: z.number().int(),
  team_ids: z.array(z.string().uuid()),
  created_at: z.string(),
  revoked_at: z.string().nullable(),
  status: z.string(),
})

export const createOrganizationInviteLinkResponseSchema = z.object({
  link: organizationInviteLinkViewSchema,
  code: z.string(),
})

export const teamViewSchema = z.object({
  id: z.string().uuid(),
  organization_id: z.string().uuid(),
  name: z.string(),
  is_default: z.boolean(),
  archived_at: z.string().nullable(),
  deleted_at: z.string().nullish(),
  role: teamRoleSchema.nullish(),
  created_at: z.string(),
  updated_at: z.string(),
})

export const organizationMemberViewSchema = z.object({
  user_id: z.string().uuid(),
  role: organizationRoleSchema,
  email: z.string(),
  display_name: z.string().nullish(),
  created_at: z.string(),
})

export const organizationAuditEventSchema = z.object({
  id: z.string().uuid(),
  organization_id: z.string().uuid(),
  actor_user_id: z.string().uuid().nullish(),
  actor_email: z.string().nullish(),
  action: z.string(),
  payload: z.unknown(),
  created_at: z.string(),
})

export type OrganizationWorkspace = z.infer<typeof organizationWorkspaceSchema>
export type TeamView = z.infer<typeof teamViewSchema>
export type OrganizationMemberView = z.infer<typeof organizationMemberViewSchema>
export type OrganizationAuditEvent = z.infer<typeof organizationAuditEventSchema>
export type OrganizationInviteLinkView = z.infer<typeof organizationInviteLinkViewSchema>
export type CreateOrganizationInviteLinkResponse = z.infer<typeof createOrganizationInviteLinkResponseSchema>

export const domainJoinModeSchema = z.enum(['auto_join', 'request'])

export const organizationDomainViewSchema = z.object({
  id: z.string().uuid(),
  organization_id: z.string().uuid(),
  domain: z.string(),
  txt_name: z.string(),
  txt_value: z.string(),
  verified_at: z.string().nullable(),
  join_mode: domainJoinModeSchema,
  created_at: z.string(),
})

export const organizationDomainJoinRequestViewSchema = z.object({
  id: z.string().uuid(),
  organization_id: z.string().uuid(),
  domain_id: z.string().uuid(),
  domain: z.string(),
  user_id: z.string().uuid(),
  email: z.string(),
  display_name: z.string().nullish(),
  status: z.string(),
  created_at: z.string(),
})

export type OrganizationDomainView = z.infer<typeof organizationDomainViewSchema>
export type OrganizationDomainJoinRequestView = z.infer<typeof organizationDomainJoinRequestViewSchema>

export const organizationSsoConfigViewSchema = z.object({
  organization_id: z.string().uuid(),
  issuer_url: z.string(),
  client_id: z.string(),
  display_name: z.string(),
  enabled: z.boolean(),
  redirect_uri: z.string(),
  created_at: z.string(),
  updated_at: z.string(),
})

export const discoverOrgSsoResponseSchema = z.object({
  sso: z.boolean(),
  organization_id: z.string().uuid().optional().nullable(),
  organization_name: z.string().optional().nullable(),
})

export type OrganizationSsoConfigView = z.infer<typeof organizationSsoConfigViewSchema>
export type DiscoverOrgSsoResponse = z.infer<typeof discoverOrgSsoResponseSchema>
export type UpsertOrganizationSso = {
  issuer_url: string
  client_id: string
  client_secret?: string
  display_name?: string
  enabled: boolean
}
