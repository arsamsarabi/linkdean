import { z } from 'zod'

export const socialSubjectTypeSchema = z.enum(['bookmark', 'category', 'dashboard'])

export const socialCommentSchema = z.object({
  id: z.string(),
  user_id: z.string(),
  author_display_name: z.string(),
  body: z.string(),
  created_at: z.string(),
  updated_at: z.string(),
  can_delete: z.boolean(),
})

export const socialSummarySchema = z.object({
  subject_type: socialSubjectTypeSchema,
  subject_id: z.string(),
  likes_count: z.number(),
  liked_by_me: z.boolean(),
  comments_count: z.number(),
  rating_avg: z.number().nullish(),
  rating_count: z.number(),
  my_rating: z.number().nullish(),
  comments: z.array(socialCommentSchema),
})

export const socialEnvelopeSchema = z.object({
  social: socialSummarySchema,
})

export type SocialSubjectType = z.infer<typeof socialSubjectTypeSchema>
export type SocialComment = z.infer<typeof socialCommentSchema>
export type SocialSummary = z.infer<typeof socialSummarySchema>
