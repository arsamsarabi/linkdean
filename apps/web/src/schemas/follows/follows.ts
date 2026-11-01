import { z } from 'zod'

export const followUserSchema = z.object({
  id: z.string(),
  display_name: z.string(),
  created_at: z.string(),
})

export const followUsersEnvelopeSchema = z.object({
  users: z.array(followUserSchema),
})

export const followRelationSchema = z.object({
  following: z.boolean(),
  followers: z.number(),
  following_count: z.number(),
})

export const followEnvelopeSchema = z.object({
  follow: followRelationSchema,
})

export type FollowUser = z.infer<typeof followUserSchema>
export type FollowRelation = z.infer<typeof followRelationSchema>
