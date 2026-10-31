import { z } from 'zod'
import {
  authTokenResponseSchema,
  bookmarkSchema,
  bootstrapStatusSchema,
  categoriesSchema,
  categorySchema,
  categorySharesSchema,
  categoryShareSchema,
  forkCategoryResultSchema,
  dashboardSchema,
  dashboardShareSchema,
  dashboardSharesSchema,
  dashboardSummariesSchema,
  duplicatesSchema,
  importResultSchema,
  meResponseSchema,
  okSchema,
  platformSettingsEnvelopeSchema,
  searchResponseSchema,
  settingsEnvelopeSchema,
  trashItemsSchema,
  trashPurgeResultSchema,
  userEnvelopeSchema,
  vaultStatusSchema,
  type Bookmark,
  type Category,
  type CategoryShare,
  type CreateDashboard,
  type Dashboard,
  type DashboardShare,
  type DashboardSummary,
  type DuplicateGroup,
  type ForkCategoryResult,
  type PatchDashboard,
  type PlatformSettings,
  type PutDashboardLayout,
  type ShareAccess,
  type TrashItem,
  type TrashKind,
  type UserPublic,
  type UserSettings,
} from '@/schemas'
import { req } from './client'

const bookmarksSchema = z.array(bookmarkSchema)

export const api = {
  bootstrap: (): Promise<{ needs_bootstrap: boolean; oidc_enabled: boolean; registration_open: boolean }> =>
    req('/auth/bootstrap-status', {}, bootstrapStatusSchema),

  getPlatformSettings: (): Promise<{ settings: PlatformSettings }> =>
    req('/admin/platform-settings', {}, platformSettingsEnvelopeSchema),

  updatePlatformSettings: (body: { registration_open?: boolean }): Promise<{ settings: PlatformSettings }> =>
    req(
      '/admin/platform-settings',
      { method: 'PATCH', body: JSON.stringify(body) },
      platformSettingsEnvelopeSchema
    ),

  register: (body: {
    email: string
    password: string
    display_name?: string
  }): Promise<{ user: UserPublic; token: string }> =>
    req('/auth/register', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  login: (body: { email: string; password: string }): Promise<{ user: UserPublic; token: string }> =>
    req('/auth/login', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  logout: (): Promise<{ ok: boolean }> => req('/auth/logout', { method: 'POST' }, okSchema),

  me: (): Promise<{ user: UserPublic; settings: UserSettings; vault_unlocked: boolean }> =>
    req('/auth/me', {}, meResponseSchema),

  completeOnboarding: (): Promise<{ user: UserPublic }> =>
    req('/auth/onboarding/complete', { method: 'POST' }, userEnvelopeSchema),

  updateProfile: (body: {
    display_name?: string | null
    bio?: string | null
    timezone?: string | null
    email?: string
  }): Promise<{ user: UserPublic }> =>
    req('/users/me', { method: 'PATCH', body: JSON.stringify(body) }, userEnvelopeSchema),

  changePassword: (body: { current_password: string; new_password: string }): Promise<{ ok: boolean }> =>
    req('/users/me/password', { method: 'POST', body: JSON.stringify(body) }, okSchema),

  getSettings: (): Promise<{ settings: UserSettings }> => req('/users/me/settings', {}, settingsEnvelopeSchema),

  updateSettings: (
    body: Partial<{
      show_hidden: boolean
      vault_search_mode: UserSettings['vault_search_mode']
      theme: UserSettings['theme']
      vault_auto_lock_minutes: number
      default_landing: UserSettings['default_landing']
      library_view: UserSettings['library_view']
      trash_retention_days: number
      show_deleted_in_library: boolean
    }>
  ): Promise<{ settings: UserSettings }> =>
    req('/users/me/settings', { method: 'PATCH', body: JSON.stringify(body) }, settingsEnvelopeSchema),

  uploadAvatar: async (blob: Blob): Promise<{ user: UserPublic }> => {
    const fd = new FormData()
    fd.append('avatar', blob, 'avatar.jpg')
    return req('/users/me/avatar', { method: 'POST', body: fd }, userEnvelopeSchema)
  },

  deleteAvatar: (): Promise<{ user: UserPublic }> => req('/users/me/avatar', { method: 'DELETE' }, userEnvelopeSchema),

  categories: (include_hidden = false, include_vault = false): Promise<Category[]> =>
    req(`/categories?include_hidden=${include_hidden}&include_vault=${include_vault}`, {}, categoriesSchema),

  createCategory: (body: Partial<Category> & { name: string }): Promise<Category> =>
    req('/categories', { method: 'POST', body: JSON.stringify(body) }, categorySchema),

  updateCategory: (
    id: string,
    body: Partial<{
      name: string
      parent_id: string | null
      is_hidden: boolean
      position: number
    }>
  ): Promise<Category> => req(`/categories/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, categorySchema),

  deleteCategory: (id: string): Promise<void> => req(`/categories/${id}`, { method: 'DELETE' }),

  forkCategory: (id: string, body: { name?: string } = {}): Promise<ForkCategoryResult> =>
    req(`/categories/${id}/fork`, { method: 'POST', body: JSON.stringify(body) }, forkCategoryResultSchema),

  categoryShares: (categoryId: string): Promise<CategoryShare[]> =>
    req(`/categories/${categoryId}/shares`, {}, categorySharesSchema),

  createCategoryShare: (
    categoryId: string,
    body: { email: string; access: ShareAccess }
  ): Promise<CategoryShare> =>
    req(
      `/categories/${categoryId}/shares`,
      { method: 'POST', body: JSON.stringify(body) },
      categoryShareSchema
    ),

  updateCategoryShare: (
    categoryId: string,
    userId: string,
    body: { access: ShareAccess }
  ): Promise<CategoryShare> =>
    req(
      `/categories/${categoryId}/shares/${userId}`,
      { method: 'PATCH', body: JSON.stringify(body) },
      categoryShareSchema
    ),

  deleteCategoryShare: (categoryId: string, userId: string): Promise<void> =>
    req(`/categories/${categoryId}/shares/${userId}`, { method: 'DELETE' }),

  bookmarks: (category_id?: string | null, include_vault = false): Promise<Bookmark[]> => {
    const q = new URLSearchParams()
    if (category_id) q.set('category_id', category_id)
    if (include_vault) q.set('include_vault', 'true')
    return req(`/bookmarks?${q}`, {}, bookmarksSchema)
  },

  createBookmark: (body: {
    url: string
    title?: string
    description?: string
    notes?: string
    category_id?: string | null
    category_ids?: string[]
    tags: string[]
  }): Promise<Bookmark> => req('/bookmarks', { method: 'POST', body: JSON.stringify(body) }, bookmarkSchema),

  updateBookmark: (
    id: string,
    body: Partial<{
      url: string
      title: string
      description: string | null
      notes: string | null
      category_ids: string[]
      tags: string[]
    }>
  ): Promise<Bookmark> => req(`/bookmarks/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, bookmarkSchema),

  deleteBookmark: (id: string): Promise<void> => req(`/bookmarks/${id}`, { method: 'DELETE' }),

  search: (q: string) => req(`/search?q=${encodeURIComponent(q)}`, {}, searchResponseSchema),

  import: (format: string, content: string): Promise<{ total: number; imported: number; failed: number }> =>
    req('/import', { method: 'POST', body: JSON.stringify({ format, content }) }, importResultSchema),

  exportUrl: (format: string): string => `/api/export?format=${format}`,

  vaultStatus: (): Promise<{ configured: boolean; unlocked: boolean }> => req('/vault/status', {}, vaultStatusSchema),

  vaultSetup: (passphrase: string): Promise<unknown> =>
    req('/vault/setup', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  vaultUnlock: (passphrase: string): Promise<unknown> =>
    req('/vault/unlock', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  vaultLock: (): Promise<unknown> => req('/vault/lock', { method: 'POST' }),

  duplicates: (): Promise<{ urls: string[]; groups: DuplicateGroup[] }> =>
    req('/bookmarks/duplicates', {}, duplicatesSchema),

  listDashboards: (): Promise<DashboardSummary[]> =>
    req('/dashboards', {}, dashboardSummariesSchema),

  createDashboard: (body: CreateDashboard = {}): Promise<Dashboard> =>
    req('/dashboards', { method: 'POST', body: JSON.stringify(body) }, dashboardSchema),

  getDefaultDashboard: (): Promise<Dashboard> => req('/dashboards/default', {}, dashboardSchema),

  putDefaultDashboard: (body: PutDashboardLayout): Promise<Dashboard> =>
    req('/dashboards/default', { method: 'PUT', body: JSON.stringify(body) }, dashboardSchema),

  getDashboard: (id: string): Promise<Dashboard> =>
    req(`/dashboards/${id}`, {}, dashboardSchema),

  putDashboard: (id: string, body: PutDashboardLayout): Promise<Dashboard> =>
    req(`/dashboards/${id}`, { method: 'PUT', body: JSON.stringify(body) }, dashboardSchema),

  patchDashboard: (id: string, body: PatchDashboard): Promise<Dashboard> =>
    req(`/dashboards/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, dashboardSchema),

  deleteDashboard: (id: string): Promise<DashboardSummary[]> =>
    req(`/dashboards/${id}`, { method: 'DELETE' }, dashboardSummariesSchema),

  dashboardShares: (dashboardId: string): Promise<DashboardShare[]> =>
    req(`/dashboards/${dashboardId}/shares`, {}, dashboardSharesSchema),

  createDashboardShare: (
    dashboardId: string,
    body: { email: string; access: ShareAccess }
  ): Promise<DashboardShare> =>
    req(
      `/dashboards/${dashboardId}/shares`,
      { method: 'POST', body: JSON.stringify(body) },
      dashboardShareSchema
    ),

  updateDashboardShare: (
    dashboardId: string,
    userId: string,
    body: { access: ShareAccess }
  ): Promise<DashboardShare> =>
    req(
      `/dashboards/${dashboardId}/shares/${userId}`,
      { method: 'PATCH', body: JSON.stringify(body) },
      dashboardShareSchema
    ),

  deleteDashboardShare: (dashboardId: string, userId: string): Promise<void> =>
    req(`/dashboards/${dashboardId}/shares/${userId}`, { method: 'DELETE' }),

  trash: (): Promise<TrashItem[]> => req('/trash', {}, trashItemsSchema),

  restoreTrashItem: (kind: TrashKind, id: string): Promise<{ ok: boolean }> =>
    req(`/trash/${kind}/${id}/restore`, { method: 'POST' }, okSchema),

  purgeTrashItem: (kind: TrashKind, id: string): Promise<{ ok: boolean }> =>
    req(`/trash/${kind}/${id}`, { method: 'DELETE' }, okSchema),

  purgeTrashSelected: (
    items: Array<{ kind: TrashKind; id: string }>
  ): Promise<{ ok: boolean; purged?: number; failed?: number }> =>
    req('/trash/purge', { method: 'POST', body: JSON.stringify({ items }) }, trashPurgeResultSchema),
}
