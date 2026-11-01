import { z } from 'zod'

export const notificationKindSchema = z.enum([
  'share_invite',
  'share_access_changed',
  'share_revoked',
  'shared_edit',
  'social_like',
  'social_comment',
  'social_rating',
  'user_followed',
  'moderation_changes',
])

export const notificationSubjectTypeSchema = z.enum(['category', 'dashboard', 'bookmark', 'user'])

export const notificationSchema = z.object({
  id: z.string(),
  kind: notificationKindSchema,
  subject_type: notificationSubjectTypeSchema,
  subject_id: z.string(),
  subject_title: z.string(),
  body: z.string(),
  payload: z.record(z.string(), z.unknown()).default({}),
  actor_user_id: z.string().nullish(),
  actor_display_name: z.string().nullish(),
  read_at: z.string().nullish(),
  created_at: z.string(),
})

export const notificationsEnvelopeSchema = z.object({
  notifications: z.array(notificationSchema),
})

export const notificationEnvelopeSchema = z.object({
  notification: notificationSchema,
})

export const unreadCountSchema = z.object({
  count: z.number(),
})

export const markAllReadSchema = z.object({
  ok: z.boolean(),
  updated: z.number().optional(),
})

export type Notification = z.infer<typeof notificationSchema>
export type NotificationKind = z.infer<typeof notificationKindSchema>
export type NotificationSubjectType = z.infer<typeof notificationSubjectTypeSchema>
