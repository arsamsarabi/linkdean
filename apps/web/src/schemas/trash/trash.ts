import { z } from 'zod'

export const trashKindSchema = z.enum(['bookmark', 'category', 'dashboard'])

export const trashItemSchema = z.object({
  kind: trashKindSchema,
  id: z.string(),
  title: z.string(),
  deleted_at: z.string(),
})

export const trashItemsSchema = z.array(trashItemSchema)

export const trashPurgeResultSchema = z.object({
  ok: z.boolean(),
  purged: z.number().optional(),
  failed: z.number().optional(),
})

export type TrashKind = z.infer<typeof trashKindSchema>
export type TrashItem = z.infer<typeof trashItemSchema>
