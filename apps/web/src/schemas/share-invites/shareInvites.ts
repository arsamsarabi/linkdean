import { z } from 'zod'

export const shareInviteSchema = z.object({
  id: z.string(),
  subject_type: z.enum(['category', 'dashboard']),
  subject_id: z.string(),
  subject_title: z.string(),
  access: z.enum(['read', 'write']),
  inviter_display_name: z.string(),
  created_at: z.string(),
  rejected_at: z.string().nullish(),
})

export const shareInvitesEnvelopeSchema = z.object({
  invites: z.array(shareInviteSchema),
})

export type ShareInvite = z.infer<typeof shareInviteSchema>
