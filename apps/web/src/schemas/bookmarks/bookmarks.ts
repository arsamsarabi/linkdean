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
  vault_id: z.string().nullish(),
  tags: z.array(z.string()),
  can_write: z.boolean(),
  shared_out: z.boolean().default(false),
  shared_with: z.array(z.string()).default([]),
  owner_display_name: z.string().nullish(),
  is_favorite: z.boolean().default(false),
  is_private: z.boolean().default(false),
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
  vault_id: z.string().nullish(),
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
  skipped: z.number().default(0),
  replaced: z.number().default(0),
  merged: z.number().default(0),
})

export const duplicateActionSchema = z.enum(['skip', 'replace', 'merge', 'keep'])

const duplicateSideSchema = z.object({
  title: z.string().nullish(),
  description: z.string().nullish(),
  tags: z.array(z.string()),
  folders: z.array(z.string()),
})

/** `POST /import/preview`: which bookmarks in a file are already in the library. */
export const importPreviewSchema = z.object({
  total: z.number(),
  new: z.number(),
  duplicates: z.array(
    z.object({
      index: z.number(),
      url: z.string(),
      existing: duplicateSideSchema.extend({ id: z.string(), updated_at: z.string() }),
      incoming: duplicateSideSchema,
    })
  ),
  folders: z.object({ reused: z.number(), created: z.number() }),
})

/** The person's own bookmark at a URL, for quick save. */
export const bookmarkLookupSchema = z.object({ bookmark: bookmarkSchema.nullable() })

/** What a page says about itself, for filling in the bookmark form. Nothing is saved. */
export const pageDetailsSchema = z.object({
  title: z.string().nullish(),
  description: z.string().nullish(),
  tags: z.array(z.string()),
})

export const duplicateGroupSchema = z.object({
  url: z.string(),
  bookmarks: z.array(bookmarkSchema),
})

export const duplicatesSchema = z.object({
  urls: z.array(z.string()),
  groups: z.array(duplicateGroupSchema).default([]),
})

export const bookmarkListPageSchema = z.object({
  items: z.array(bookmarkSchema),
  next_cursor: z.string().nullable(),
  total: z.number(),
})

export const bookmarkIdsSchema = z.object({ ids: z.array(z.string()) })

/** `GET /tags` rows; the library filter only needs the names. */
export const tagNamesSchema = z.array(z.object({ name: z.string() }))

export type BookmarkListPage = z.infer<typeof bookmarkListPageSchema>
export type BookmarkIdsResponse = z.infer<typeof bookmarkIdsSchema>
export type CategoryRef = z.infer<typeof categoryRefSchema>
export type Bookmark = z.infer<typeof bookmarkSchema>
export type LinkCheckStatus = z.infer<typeof linkCheckStatusSchema>
export type SearchHit = z.infer<typeof searchHitSchema>
export type SearchResponse = z.infer<typeof searchResponseSchema>
export type ImportResult = z.infer<typeof importResultSchema>
export type DuplicateAction = z.infer<typeof duplicateActionSchema>
export type ImportPreview = z.infer<typeof importPreviewSchema>
export type ImportDuplicate = ImportPreview['duplicates'][number]
/** The import body's `duplicates`: an action for all rows, and the rows that differ. */
export type DuplicateDecisions = { default: DuplicateAction; overrides: Record<number, DuplicateAction> }
export type PageDetails = z.infer<typeof pageDetailsSchema>
export type DuplicateGroup = z.infer<typeof duplicateGroupSchema>
export type DuplicatesResponse = z.infer<typeof duplicatesSchema>
