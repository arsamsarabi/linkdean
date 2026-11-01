import { z } from 'zod'
import { publicBookmarkSchema } from '../categories'
import { followRelationSchema } from '../follows'

export const profileVisibilitySchema = z.enum(['members', 'followers', 'hidden'])
export const categoryVisibilitySchema = z.enum(['private', 'followers', 'members'])

export const profileSchema = z.object({
  id: z.string(),
  display_name: z.string(),
  bio: z.string().nullish(),
  has_avatar: z.boolean(),
  is_self: z.boolean(),
  follow: followRelationSchema,
  email: z.string().nullish(),
  profile_visibility: profileVisibilitySchema.nullish(),
})

export const profileSummarySchema = z.object({
  id: z.string(),
  display_name: z.string(),
  has_avatar: z.boolean(),
})

export const profileSummariesEnvelopeSchema = z.object({
  users: z.array(profileSummarySchema),
})

export const profileBookmarksPageSchema = z.object({
  bookmarks: z.array(publicBookmarkSchema),
  total: z.number(),
})

export type ProfileVisibility = z.infer<typeof profileVisibilitySchema>
export type CategoryVisibility = z.infer<typeof categoryVisibilitySchema>
export type Profile = z.infer<typeof profileSchema>
export type ProfileSummary = z.infer<typeof profileSummarySchema>
export type ProfileBookmarksPage = z.infer<typeof profileBookmarksPageSchema>
