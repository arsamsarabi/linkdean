import { z } from 'zod'

export const activityActionSchema = z.enum(['created', 'updated', 'deleted', 'restored'])

export const activitySubjectTypeSchema = z.enum(['bookmark', 'category', 'dashboard'])

export const activityEventSchema = z.object({
  id: z.string(),
  action: activityActionSchema,
  subject_type: activitySubjectTypeSchema,
  subject_id: z.string(),
  subject_title: z.string(),
  summary: z.string(),
  payload: z.record(z.string(), z.unknown()).default({}),
  created_at: z.string(),
})

export const activityEventsEnvelopeSchema = z.object({
  events: z.array(activityEventSchema),
})

export type ActivityEvent = z.infer<typeof activityEventSchema>
export type ActivityAction = z.infer<typeof activityActionSchema>
export type ActivitySubjectType = z.infer<typeof activitySubjectTypeSchema>
