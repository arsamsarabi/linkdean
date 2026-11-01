import { z } from 'zod'

export const blockRelationSchema = z.object({
  blocked: z.boolean(),
})

export const blockEnvelopeSchema = z.object({
  block: blockRelationSchema,
})

export const muteRelationSchema = z.object({
  muted: z.boolean(),
})

export const muteEnvelopeSchema = z.object({
  mute: muteRelationSchema,
})

export type BlockRelation = z.infer<typeof blockRelationSchema>
export type MuteRelation = z.infer<typeof muteRelationSchema>
