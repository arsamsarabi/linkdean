import { z } from 'zod'
import { bookmarkSchema } from '@/schemas/bookmarks'
import {
  categoryAccessSchema,
  inviteEmailFields,
  pendingShareInviteSchema,
  shareAccessSchema,
} from '@/schemas/categories'

export const dashboardItemSchema = z.object({
  id: z.string(),
  position: z.number(),
  bookmark: bookmarkSchema,
  /** The viewer's opens: last 30 days in a section counting that window, else all time. */
  open_count: z.number().nullish(),
  last_opened_at: z.string().nullish(),
})

export const dashboardSectionKindSchema = z.enum(['pinned', 'latest', 'most_opened', 'recently_visited'])
export const dashboardSectionDisplaySchema = z.enum(['small', 'medium', 'large', 'icons'])
export const dashboardOpenWindowSchema = z.enum(['last_30_days', 'all_time'])

export const DASHBOARD_SECTION_MIN_ITEMS = 4
export const DASHBOARD_SECTION_MAX_ITEMS = 24
export const DASHBOARD_SECTION_DEFAULT_ITEMS = 8

export const dashboardSectionSourceSchema = z.object({
  category_id: z.string().nullish(),
  tag: z.string().nullish(),
  site: z.string().nullish(),
  favorites: z.boolean().default(false),
  unread: z.boolean().default(false),
})

/** A section's type, display mode and source; older servers send none, meaning pinned and medium. */
export const dashboardSectionSettingsSchema = z.object({
  kind: dashboardSectionKindSchema.default('pinned'),
  display: dashboardSectionDisplaySchema.default('medium'),
  item_limit: z.number().default(DASHBOARD_SECTION_DEFAULT_ITEMS),
  source: dashboardSectionSourceSchema.default({ favorites: false, unread: false }),
  open_window: dashboardOpenWindowSchema.default('all_time'),
})

export const dashboardSectionSchema = z
  .object({
    id: z.string(),
    title: z.string(),
    position: z.number(),
    items: z.array(dashboardItemSchema),
  })
  .extend(dashboardSectionSettingsSchema.shape)

export const dashboardSchema = z.object({
  id: z.string(),
  owner_user_id: z.string(),
  title: z.string(),
  is_default: z.boolean(),
  sections: z.array(dashboardSectionSchema),
  access: categoryAccessSchema.default('owner'),
  shared_out: z.boolean().default(false),
  shared_with: z.array(z.string()).default([]),
  owner_display_name: z.string().nullish(),
  deleted_at: z.string().nullish(),
  created_at: z.string(),
  updated_at: z.string(),
})

export const dashboardSummarySchema = z.object({
  id: z.string(),
  owner_user_id: z.string(),
  title: z.string(),
  is_default: z.boolean(),
  access: categoryAccessSchema.default('owner'),
  shared_out: z.boolean().default(false),
  shared_with: z.array(z.string()).default([]),
  owner_display_name: z.string().nullish(),
  deleted_at: z.string().nullish(),
  created_at: z.string(),
  updated_at: z.string(),
})

export const dashboardShareSchema = z.object({
  id: z.string(),
  dashboard_id: z.string(),
  invitee_user_id: z.string(),
  invitee_email: z.string(),
  invitee_display_name: z.string().nullish(),
  access: shareAccessSchema,
  invited_by_user_id: z.string(),
  created_at: z.string().optional(),
  updated_at: z.string().optional(),
})

export const dashboardSharesSchema = z.array(dashboardShareSchema)

export const dashboardSharesEnvelopeSchema = z.object({
  shares: z.array(dashboardShareSchema),
  pending: z.array(pendingShareInviteSchema).default([]),
})

export const createDashboardShareOutcomeSchema = z.discriminatedUnion('status', [
  z.object({ status: z.literal('active'), share: dashboardShareSchema, ...inviteEmailFields }),
  z.object({ status: z.literal('pending'), invite: pendingShareInviteSchema, ...inviteEmailFields }),
])

export const dashboardSummariesSchema = z.array(dashboardSummarySchema)

export const putDashboardLayoutSchema = z.object({
  title: z.string().optional(),
  sections: z.array(
    z
      .object({
        title: z.string(),
        bookmark_ids: z.array(z.string()),
      })
      .extend(dashboardSectionSettingsSchema.partial().shape)
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
export type DashboardSectionKind = z.infer<typeof dashboardSectionKindSchema>
export type DashboardSectionDisplay = z.infer<typeof dashboardSectionDisplaySchema>
export type DashboardOpenWindow = z.infer<typeof dashboardOpenWindowSchema>
export type DashboardSectionSource = z.infer<typeof dashboardSectionSourceSchema>
export type DashboardSectionSettings = z.infer<typeof dashboardSectionSettingsSchema>
export type Dashboard = z.infer<typeof dashboardSchema>
export type DashboardSummary = z.infer<typeof dashboardSummarySchema>
export type DashboardShare = z.infer<typeof dashboardShareSchema>
export type DashboardSharesEnvelope = z.infer<typeof dashboardSharesEnvelopeSchema>
export type CreateDashboardShareOutcome = z.infer<typeof createDashboardShareOutcomeSchema>
export type PutDashboardLayout = z.infer<typeof putDashboardLayoutSchema>
export type CreateDashboard = z.infer<typeof createDashboardSchema>
export type PatchDashboard = z.infer<typeof patchDashboardSchema>
