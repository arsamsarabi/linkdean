import { z } from 'zod'

export const organizationRoleSchema = z.enum(['owner', 'admin', 'member', 'guest'])
export const teamRoleSchema = z.enum(['maintainer', 'editor', 'viewer'])

export const organizationWorkspaceSchema = z.object({
  id: z.string().uuid(),
  name: z.string(),
  role: organizationRoleSchema,
  deleted_at: z.string().nullish(),
  created_at: z.string(),
  updated_at: z.string(),
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
