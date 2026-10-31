import { z } from 'zod'

export const categoryRefSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
})

export const bookmarkSchema = z.object({
  id: z.string(),
  category_id: z.string().nullish(),
  categories: z.array(categoryRefSchema),
  url: z.string(),
  title: z.string(),
  description: z.string().nullish(),
  notes: z.string().nullish(),
  favicon_url: z.string().nullish(),
  preview_image_url: z.string().nullish(),
  has_preview: z.boolean(),
  is_vault: z.boolean(),
  tags: z.array(z.string()),
  created_at: z.string(),
  updated_at: z.string(),
})

export const searchHitSchema = z.object({
  id: z.string(),
  title: z.string(),
  url: z.string(),
  description: z.string().nullish(),
  favicon_url: z.string().nullish(),
  preview_image_url: z.string().nullish(),
  has_preview: z.boolean(),
  tags: z.array(z.string()),
  category_id: z.string().nullish(),
  categories: z.array(categoryRefSchema).optional(),
  is_vault: z.boolean(),
})

export const searchResponseSchema = z.object({
  hits: z.array(searchHitSchema),
  vault_prompt: z.boolean(),
})

export const importResultSchema = z.object({
  total: z.number(),
  imported: z.number(),
  failed: z.number(),
})

export const duplicatesSchema = z.object({
  urls: z.array(z.string()),
})

export type CategoryRef = z.infer<typeof categoryRefSchema>
export type Bookmark = z.infer<typeof bookmarkSchema>
export type SearchHit = z.infer<typeof searchHitSchema>
export type SearchResponse = z.infer<typeof searchResponseSchema>
export type ImportResult = z.infer<typeof importResultSchema>
