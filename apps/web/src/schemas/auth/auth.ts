import { z } from 'zod'

export const userPublicSchema = z.object({
  id: z.string(),
  email: z.string(),
  display_name: z.string().nullish(),
  bio: z.string().nullish(),
  timezone: z.string().nullish(),
  has_avatar: z.boolean(),
  has_password: z.boolean(),
  has_vault: z.boolean(),
  is_admin: z.boolean(),
  onboarding_completed: z.boolean(),
})

export const userSettingsSchema = z.object({
  user_id: z.string(),
  show_hidden: z.boolean(),
  vault_search_mode: z.enum(['never', 'when_unlocked', 'always']),
  theme: z.enum(['light', 'dark', 'system']),
  vault_auto_lock_minutes: z.number(),
  default_landing: z.enum(['/', '/vault', '/import']),
  library_view: z.enum(['grid', 'explorer']).default('grid'),
  updated_at: z.string(),
})

export const meResponseSchema = z.object({
  user: userPublicSchema,
  settings: userSettingsSchema,
  vault_unlocked: z.boolean(),
})

export const bootstrapStatusSchema = z.object({
  needs_bootstrap: z.boolean(),
  oidc_enabled: z.boolean(),
})

export const authTokenResponseSchema = z.object({
  user: userPublicSchema,
  token: z.string(),
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
