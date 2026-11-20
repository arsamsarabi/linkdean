import { z } from 'zod'

export const savedSearchFiltersSchema = z.object({
  q: z.string().optional(),
  sort: z.string().optional(),
  category: z.array(z.string()).default([]),
  tag: z.array(z.string()).default([]),
  flag: z.array(z.string()).default([]),
  since: z.string().optional(),
  vault: z.boolean().optional(),
  vault_id: z.string().optional(),
  folder: z.string().optional(),
})

export const savedSearchSchema = z.object({
  id: z.string(),
  name: z.string(),
  filters: savedSearchFiltersSchema,
  created_at: z.string(),
  updated_at: z.string(),
})

export type SavedSearchFilters = z.infer<typeof savedSearchFiltersSchema>
export type SavedSearch = z.infer<typeof savedSearchSchema>
