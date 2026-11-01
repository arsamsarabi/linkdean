import { z } from 'zod'

export const categoryRefSchema = z.object({
  id: z.string(),
  name: z.string(),
  path: z.string(),
})

export const linkCheckStatusSchema = z.enum(['ok', 'broken', 'unreachable', 'unverifiable'])

export const bookmarkSchema = z.object({
  id: z.string(),
  owner_user_id: z.string(),
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
  can_write: z.boolean(),
  shared_out: z.boolean().default(false),
  shared_with: z.array(z.string()).default([]),
  owner_display_name: z.string().nullish(),
  is_favorite: z.boolean().default(false),
  is_unread: z.boolean().default(false),
  is_pinned: z.boolean().default(false),
  deleted_at: z.string().nullish(),
  link_check_status: linkCheckStatusSchema.nullish(),
  link_check_status_code: z.number().int().nullish(),
  link_check_error: z.string().nullish(),
  link_checked_at: z.string().nullish(),
  created_at: z.string(),
  updated_at: z.string(),
})

export const searchHitSchema = z.object({
  id: z.string(),
  owner_user_id: z.string(),
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
  can_write: z.boolean(),
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

export const duplicateGroupSchema = z.object({
  url: z.string(),
  bookmarks: z.array(bookmarkSchema),
})

export const duplicatesSchema = z.object({
  urls: z.array(z.string()),
  groups: z.array(duplicateGroupSchema).default([]),
})

export type CategoryRef = z.infer<typeof categoryRefSchema>
export type Bookmark = z.infer<typeof bookmarkSchema>
export type LinkCheckStatus = z.infer<typeof linkCheckStatusSchema>
export type SearchHit = z.infer<typeof searchHitSchema>
export type SearchResponse = z.infer<typeof searchResponseSchema>
export type ImportResult = z.infer<typeof importResultSchema>
export type DuplicateGroup = z.infer<typeof duplicateGroupSchema>
export type DuplicatesResponse = z.infer<typeof duplicatesSchema>
