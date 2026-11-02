import { z } from 'zod'

export const categoryAccessSchema = z.enum(['owner', 'read', 'write'])

export const categorySchema = z.object({
  id: z.string(),
  owner_user_id: z.string().optional(),
  parent_id: z.string().nullish(),
  name: z.string(),
  is_hidden: z.boolean(),
  visibility: z.enum(['private', 'followers', 'members']).default('private'),
  is_vault: z.boolean(),
  vault_id: z.string().nullish(),
  icon: z.string().nullish(),
  position: z.number(),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
  access: categoryAccessSchema.default('owner'),
  shared_out: z.boolean().default(false),
  shared_with: z.array(z.string()).default([]),
  owner_display_name: z.string().nullish(),
  deleted_at: z.string().nullish(),
})

export const categoriesSchema = z.array(categorySchema)

export const forkCategoryResultSchema = z.object({
  category: categorySchema,
  bookmarks_copied: z.number(),
})

export type CategoryAccess = z.infer<typeof categoryAccessSchema>
export type Category = z.infer<typeof categorySchema>
export type ForkCategoryResult = z.infer<typeof forkCategoryResultSchema>

export const shareAccessSchema = z.enum(['read', 'write'])

export const categoryShareSchema = z.object({
  id: z.string(),
  category_id: z.string(),
  invitee_user_id: z.string(),
  invitee_email: z.string(),
  invitee_display_name: z.string().nullish(),
  access: shareAccessSchema,
  invited_by_user_id: z.string(),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
})

export const pendingShareInviteSchema = z.object({
  id: z.string(),
  subject_type: z.enum(['category', 'dashboard']),
  subject_id: z.string(),
  invitee_email: z.string(),
  access: shareAccessSchema,
  invited_by_user_id: z.string(),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
  claim_token: z.string().optional(),
})

export const categorySharesEnvelopeSchema = z.object({
  shares: z.array(categoryShareSchema),
  pending: z.array(pendingShareInviteSchema).default([]),
})

export const createCategoryShareOutcomeSchema = z.discriminatedUnion('status', [
  z.object({ status: z.literal('active'), share: categoryShareSchema }),
  z.object({ status: z.literal('pending'), invite: pendingShareInviteSchema }),
])

/** @deprecated Prefer categorySharesEnvelopeSchema */
export const categorySharesSchema = z.array(categoryShareSchema)

export type ShareAccess = z.infer<typeof shareAccessSchema>
export type CategoryShare = z.infer<typeof categoryShareSchema>
export type PendingShareInvite = z.infer<typeof pendingShareInviteSchema>
export type CategorySharesEnvelope = z.infer<typeof categorySharesEnvelopeSchema>
export type CreateCategoryShareOutcome = z.infer<typeof createCategoryShareOutcomeSchema>

export const categoryPublicLinkSchema = z.object({
  enabled: z.boolean(),
  slug: z.string().nullish(),
  path: z.string().nullish(),
})

export const publicBookmarkSchema = z.object({
  id: z.string(),
  url: z.string(),
  title: z.string(),
  description: z.string().nullish(),
  favicon_url: z.string().nullish(),
  preview_image_url: z.string().nullish(),
  has_preview: z.boolean(),
  preview_url: z.string().nullish(),
  tags: z.array(z.string()),
})

export const publicListSchema = z.object({
  name: z.string(),
  icon: z.string().nullish(),
  owner_display_name: z.string(),
  bookmarks: z.array(publicBookmarkSchema),
  /** All bookmarks on the list; `bookmarks` holds at most the newest 500. */
  total: z.number().optional(),
})

export type CategoryPublicLink = z.infer<typeof categoryPublicLinkSchema>
export type PublicBookmark = z.infer<typeof publicBookmarkSchema>
export type PublicList = z.infer<typeof publicListSchema>
