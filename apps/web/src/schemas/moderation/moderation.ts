import { z } from 'zod'

export const banNoticeSchema = z.object({
  reason: z.string(),
  ends_at: z.string().nullish(),
})

export const banViewSchema = z.object({
  user_id: z.string(),
  email: z.string(),
  display_name: z.string().nullish(),
  is_admin: z.boolean(),
  reason: z.string(),
  ends_at: z.string().nullish(),
  created_by_email: z.string().nullish(),
  created_at: z.string(),
})

export const bansEnvelopeSchema = z.object({
  bans: z.array(banViewSchema),
})

export const moderationActionSchema = z.object({
  id: z.string(),
  action: z.string(),
  admin_email: z.string().nullish(),
  subject_type: z.string().nullish(),
  subject_id: z.string().nullish(),
  subject_title: z.string(),
  reason: z.string(),
  created_at: z.string(),
})

export const moderationActionsEnvelopeSchema = z.object({
  actions: z.array(moderationActionSchema),
})

const exposedCategorySchema = z.object({
  id: z.string(),
  name: z.string(),
  visibility: z.string(),
  public_slug: z.string().nullish(),
})

const exposedBookmarkSchema = z.object({
  id: z.string(),
  title: z.string(),
  url: z.string(),
})

const exposedShareSchema = z.object({
  kind: z.string(),
  id: z.string(),
  title: z.string(),
  invitee: z.string(),
  access: z.string(),
})

const exposedCommentSchema = z.object({
  id: z.string(),
  body: z.string(),
  subject_type: z.string(),
  created_at: z.string(),
})

export const exposedContentSchema = z.object({
  public_links: z.array(exposedCategorySchema),
  categories: z.array(exposedCategorySchema),
  bookmarks: z.array(exposedBookmarkSchema),
  profile_visibility: z.string().nullish(),
  shares: z.array(exposedShareSchema),
  comments: z.array(exposedCommentSchema),
})

export const exposedContentEnvelopeSchema = z.object({
  content: exposedContentSchema,
})

export const moderateOutcomeEnvelopeSchema = z.object({
  outcome: z.object({
    applied: z.number(),
    skipped: z.number(),
  }),
})

export type BanNotice = z.infer<typeof banNoticeSchema>
export type BanView = z.infer<typeof banViewSchema>
export type ModerationAction = z.infer<typeof moderationActionSchema>
export type ExposedContent = z.infer<typeof exposedContentSchema>
export type ModerateOutcome = z.infer<typeof moderateOutcomeEnvelopeSchema>['outcome']

export type BanDurationBody = { days?: number; ends_at?: string; permanent?: boolean }

export type ModerateItem = {
  action: string
  subject_type?: string
  subject_id: string
}
