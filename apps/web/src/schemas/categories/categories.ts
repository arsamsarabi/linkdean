import { z } from 'zod'

export const categorySchema = z.object({
  id: z.string(),
  parent_id: z.string().nullish(),
  name: z.string(),
  is_hidden: z.boolean(),
  is_vault: z.boolean(),
  position: z.number(),
})

export const categoriesSchema = z.array(categorySchema)

export type Category = z.infer<typeof categorySchema>
