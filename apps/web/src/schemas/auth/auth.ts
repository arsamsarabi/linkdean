import { z } from 'zod'

export const userPublicSchema = z.object({
  id: z.string(),
  email: z.string(),
  display_name: z.string().nullish(),
  bio: z.string().nullish(),
  timezone: z.string().nullish(),
  has_avatar: z.boolean(),
  profile_visibility: z.enum(['members', 'followers', 'hidden']).default('hidden'),
  has_password: z.boolean(),
  has_vault: z.boolean(),
  is_admin: z.boolean(),
  onboarding_completed: z.boolean(),
  api_keys_enabled: z.boolean().default(true),
})

export const userSettingsSchema = z.object({
  user_id: z.string(),
  show_hidden: z.boolean(),
  vault_search_mode: z.enum(['never', 'when_unlocked', 'always']),
  theme: z.enum(['light', 'dark', 'system']),
  vault_auto_lock_minutes: z.number(),
  default_landing: z.enum(['/', '/library']),
  library_view: z.enum(['grid', 'list', 'explorer']).default('grid'),
  trash_retention_days: z.number().default(30),
  show_deleted_in_library: z.boolean().default(false),
  show_vault_in_library: z.boolean().default(true),
  vault_delete_mode: z.enum(['trash', 'permanent']).default('trash'),
  updated_at: z.string(),
})

export const meResponseSchema = z.object({
  user: userPublicSchema,
  settings: userSettingsSchema,
  vault_unlocked: z.boolean(),
  api_keys_allowed: z.boolean().default(true),
})

export const bootstrapStatusSchema = z.object({
  needs_bootstrap: z.boolean(),
  /** The server has a BOOTSTRAP_TOKEN, so the first account must enter it. */
  bootstrap_token_required: z.boolean().default(false),
  oidc_enabled: z.boolean(),
  registration_open: z.boolean().default(true),
  /** Outbound mail is on, so a forgotten password can be reset by email. */
  mail_enabled: z.boolean().default(false),
})

export const platformSettingsSchema = z.object({
  registration_open: z.boolean(),
  api_keys_enabled: z.boolean().default(true),
  updated_at: z.string(),
})

export const platformSettingsEnvelopeSchema = z.object({
  settings: platformSettingsSchema,
})

export const apiKeySchema = z.object({
  id: z.string(),
  name: z.string(),
  prefix: z.string(),
  last_used_at: z.string().nullish(),
  revoked_at: z.string().nullish(),
  created_at: z.string(),
})

export const apiKeysEnvelopeSchema = z.object({
  keys: z.array(apiKeySchema),
})

export const createApiKeyResponseSchema = z.object({
  key: apiKeySchema,
  token: z.string(),
})

export const adminUserSummarySchema = z.object({
  id: z.string(),
  email: z.string(),
  display_name: z.string().nullish(),
  is_admin: z.boolean(),
  api_keys_enabled: z.boolean(),
  created_at: z.string(),
  banned: z.boolean().default(false),
  banned_until: z.string().nullish(),
  /** False for SSO-only accounts, which cannot get a reset link. */
  has_password: z.boolean().default(true),
})

export const adminUsersEnvelopeSchema = z.object({
  users: z.array(adminUserSummarySchema),
})

export const adminUserEnvelopeSchema = z.object({
  user: adminUserSummarySchema,
})

export const adminReindexSchema = z.object({
  ok: z.boolean(),
  upserted: z.number(),
  failed: z.number(),
})

export const registrationInviteSchema = z.object({
  id: z.string(),
  code_prefix: z.string(),
  email: z.string().nullish(),
  created_by_user_id: z.string(),
  expires_at: z.string(),
  used_at: z.string().nullish(),
  used_by_user_id: z.string().nullish(),
  revoked_at: z.string().nullish(),
  created_at: z.string(),
  status: z.string(),
})

export const registrationInvitesEnvelopeSchema = z.object({
  invites: z.array(registrationInviteSchema),
})

export const createRegistrationInviteResponseSchema = z.object({
  invite: registrationInviteSchema,
  code: z.string(),
})

export const authTokenResponseSchema = z.object({
  user: userPublicSchema,
  token: z.string(),
})

export const forgotPasswordResponseSchema = z.object({
  ok: z.boolean(),
  /** How long the emailed link works. */
  minutes: z.number(),
})

export const adminResetLinkSchema = z.object({
  url: z.string(),
  expires_in_hours: z.number(),
})

export const userEnvelopeSchema = z.object({
  user: userPublicSchema,
})

export const settingsEnvelopeSchema = z.object({
  settings: userSettingsSchema,
})

export const okSchema = z.object({
  ok: z.boolean(),
})

export type UserPublic = z.infer<typeof userPublicSchema>
export type UserSettings = z.infer<typeof userSettingsSchema>
export type MeResponse = z.infer<typeof meResponseSchema>
export type BootstrapStatus = z.infer<typeof bootstrapStatusSchema>
export type AuthTokenResponse = z.infer<typeof authTokenResponseSchema>
export type PlatformSettings = z.infer<typeof platformSettingsSchema>
export type ApiKey = z.infer<typeof apiKeySchema>
export type CreateApiKeyResponse = z.infer<typeof createApiKeyResponseSchema>
export type AdminUserSummary = z.infer<typeof adminUserSummarySchema>
export type ForgotPasswordResponse = z.infer<typeof forgotPasswordResponseSchema>
export type AdminResetLink = z.infer<typeof adminResetLinkSchema>
export type RegistrationInvite = z.infer<typeof registrationInviteSchema>
export type CreateRegistrationInviteResponse = z.infer<typeof createRegistrationInviteResponseSchema>
