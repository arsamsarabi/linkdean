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

export const putDashboardLayoutSchema = z.object({
  title: z.string().optional(),
  sections: z.array(
    z.object({
      title: z.string(),
      bookmark_ids: z.array(z.string()),
    })
  ),
})

export type DashboardItem = z.infer<typeof dashboardItemSchema>
export type DashboardSection = z.infer<typeof dashboardSectionSchema>
export type Dashboard = z.infer<typeof dashboardSchema>
export type PutDashboardLayout = z.infer<typeof putDashboardLayoutSchema>
