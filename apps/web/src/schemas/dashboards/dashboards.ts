import { z } from 'zod'
import { bookmarkSchema } from '@/schemas/bookmarks'

export const dashboardItemSchema = z.object({
  id: z.string(),
  position: z.number(),
  bookmark: bookmarkSchema,
})

export const dashboardSectionSchema = z.object({
  id: z.string(),
  title: z.string(),
  position: z.number(),
  items: z.array(dashboardItemSchema),
})

export const dashboardSchema = z.object({
  id: z.string(),
  title: z.string(),
  is_default: z.boolean(),
  sections: z.array(dashboardSectionSchema),
  created_at: z.string(),
  updated_at: z.string(),
})

export const dashboardSummarySchema = z.object({
  id: z.string(),
  title: z.string(),
  is_default: z.boolean(),
  created_at: z.string(),
  updated_at: z.string(),
})

export const dashboardSummariesSchema = z.array(dashboardSummarySchema)

export const putDashboardLayoutSchema = z.object({
  title: z.string().optional(),
  sections: z.array(
    z.object({
      title: z.string(),
      bookmark_ids: z.array(z.string()),
    })
  ),
})

export const createDashboardSchema = z.object({
  title: z.string().optional(),
})

export const patchDashboardSchema = z.object({
  title: z.string().optional(),
  is_default: z.boolean().optional(),
})

export type DashboardItem = z.infer<typeof dashboardItemSchema>
export type DashboardSection = z.infer<typeof dashboardSectionSchema>
export type Dashboard = z.infer<typeof dashboardSchema>
export type DashboardSummary = z.infer<typeof dashboardSummarySchema>
export type PutDashboardLayout = z.infer<typeof putDashboardLayoutSchema>
export type CreateDashboard = z.infer<typeof createDashboardSchema>
export type PatchDashboard = z.infer<typeof patchDashboardSchema>
