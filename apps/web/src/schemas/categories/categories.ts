import { z } from 'zod'

export const categoryAccessSchema = z.enum(['owner', 'read', 'write'])

export const categorySchema = z.object({
  id: z.string(),
  owner_user_id: z.string().optional(),
  parent_id: z.string().nullish(),
  name: z.string(),
  is_hidden: z.boolean(),
  is_vault: z.boolean(),
  position: z.number(),
  access: categoryAccessSchema.default('owner'),
  shared_out: z.boolean().default(false),
  shared_with: z.array(z.string()).default([]),
  owner_display_name: z.string().nullish(),
  deleted_at: z.string().nullish(),
})

export const categoriesSchema = z.array(categorySchema)

export const forkCategoryResultSchema = z.object({
  category: categorySchema,
  bookmarks_copied: z.number(),
})

export type CategoryAccess = z.infer<typeof categoryAccessSchema>
export type Category = z.infer<typeof categorySchema>
export type ForkCategoryResult = z.infer<typeof forkCategoryResultSchema>

export const shareAccessSchema = z.enum(['read', 'write'])

export const categoryShareSchema = z.object({
  id: z.string(),
  category_id: z.string(),
  invitee_user_id: z.string(),
  invitee_email: z.string(),
  invitee_display_name: z.string().nullish(),
  access: shareAccessSchema,
  invited_by_user_id: z.string(),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
})

export const categorySharesSchema = z.array(categoryShareSchema)

export type ShareAccess = z.infer<typeof shareAccessSchema>
export type CategoryShare = z.infer<typeof categoryShareSchema>
