import { z } from 'zod'
import {
  activityEventsEnvelopeSchema,
  adminReindexSchema,
  adminUserEnvelopeSchema,
  adminUsersEnvelopeSchema,
  apiKeysEnvelopeSchema,
  authTokenResponseSchema,
  blockEnvelopeSchema,
  bookmarkSchema,
  bootstrapStatusSchema,
  categoriesSchema,
  categoryPublicLinkSchema,
  categorySchema,
  categoryShareSchema,
  categorySharesEnvelopeSchema,
  createApiKeyResponseSchema,
  createCategoryShareOutcomeSchema,
  createDashboardShareOutcomeSchema,
  createdVaultSchema,
  createRegistrationInviteResponseSchema,
  dashboardSchema,
  dashboardShareSchema,
  dashboardSharesEnvelopeSchema,
  dashboardSummariesSchema,
  duplicatesSchema,
  followEnvelopeSchema,
  followUsersEnvelopeSchema,
  forkCategoryResultSchema,
  globalPassphraseSchema,
  importResultSchema,
  markAllReadSchema,
  meResponseSchema,
  muteEnvelopeSchema,
  notificationEnvelopeSchema,
  notificationsEnvelopeSchema,
  okSchema,
  pendingShareInviteSchema,
  platformSettingsEnvelopeSchema,
  profileBookmarksPageSchema,
  profileSchema,
  profileSummariesEnvelopeSchema,
  publicListSchema,
  recoveryKeySchema,
  registrationInvitesEnvelopeSchema,
  searchResponseSchema,
  settingsEnvelopeSchema,
  shareInvitesEnvelopeSchema,
  socialEnvelopeSchema,
  trashItemsSchema,
  trashPurgeResultSchema,
  unreadCountSchema,
  userEnvelopeSchema,
  vaultsEnvelopeSchema,
  type ActivityAction,
  type ActivityEvent,
  type ActivitySubjectType,
  type AdminUserSummary,
  type ApiKey,
  type BlockRelation,
  type Bookmark,
  type Category,
  type CategoryPublicLink,
  type CategoryShare,
  type CategorySharesEnvelope,
  type CategoryVisibility,
  type CreateApiKeyResponse,
  type CreateCategoryShareOutcome,
  type CreateDashboard,
  type CreateDashboardShareOutcome,
  type CreateRegistrationInviteResponse,
  type Dashboard,
  type DashboardShare,
  type DashboardSharesEnvelope,
  type DashboardSummary,
  type DuplicateGroup,
  type FollowRelation,
  type FollowUser,
  type ForkCategoryResult,
  type MuteRelation,
  type Notification,
  type PatchDashboard,
  type PendingShareInvite,
  type PlatformSettings,
  type Profile,
  type ProfileBookmarksPage,
  type ProfileSummary,
  type ProfileVisibility,
  type PublicList,
  type PutDashboardLayout,
  type RegistrationInvite,
  type ShareAccess,
  type ShareInvite,
  type SocialSubjectType,
  type SocialSummary,
  type TrashItem,
  type TrashKind,
  type UserPublic,
  type UserSettings,
  type Vault,
} from '@/schemas'
import { req } from './client'

const bookmarksSchema = z.array(bookmarkSchema)

export const api = {
  bootstrap: (): Promise<{ needs_bootstrap: boolean; oidc_enabled: boolean; registration_open: boolean }> =>
    req('/auth/bootstrap-status', {}, bootstrapStatusSchema),

  getPlatformSettings: (): Promise<{ settings: PlatformSettings }> =>
    req('/admin/platform-settings', {}, platformSettingsEnvelopeSchema),

  updatePlatformSettings: (body: {
    registration_open?: boolean
    api_keys_enabled?: boolean
  }): Promise<{ settings: PlatformSettings }> =>
    req('/admin/platform-settings', { method: 'PATCH', body: JSON.stringify(body) }, platformSettingsEnvelopeSchema),

  listAdminUsers: (): Promise<{ users: AdminUserSummary[] }> => req('/admin/users', {}, adminUsersEnvelopeSchema),

  updateAdminUser: (
    id: string,
    body: { api_keys_enabled?: boolean; is_admin?: boolean }
  ): Promise<{ user: AdminUserSummary }> =>
    req(`/admin/users/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, adminUserEnvelopeSchema),

  reindexSearch: (): Promise<{ ok: boolean; upserted: number; failed: number }> =>
    req('/admin/search/reindex', { method: 'POST' }, adminReindexSchema),

  listRegistrationInvites: (): Promise<{ invites: RegistrationInvite[] }> =>
    req('/admin/invites', {}, registrationInvitesEnvelopeSchema),

  createRegistrationInvite: (body: {
    email?: string
    expires_in_days?: number
  }): Promise<CreateRegistrationInviteResponse> =>
    req('/admin/invites', { method: 'POST', body: JSON.stringify(body) }, createRegistrationInviteResponseSchema),

  revokeRegistrationInvite: (id: string): Promise<{ ok: boolean }> =>
    req(`/admin/invites/${id}`, { method: 'DELETE' }, okSchema),

  listApiKeys: (): Promise<{ keys: ApiKey[] }> => req('/api-keys', {}, apiKeysEnvelopeSchema),

  createApiKey: (body: { name: string }): Promise<CreateApiKeyResponse> =>
    req('/api-keys', { method: 'POST', body: JSON.stringify(body) }, createApiKeyResponseSchema),

  revokeApiKey: (id: string): Promise<{ ok: boolean }> => req(`/api-keys/${id}`, { method: 'DELETE' }, okSchema),

  listActivity: (opts?: {
    limit?: number
    subjectType?: ActivitySubjectType
    action?: ActivityAction
  }): Promise<{ events: ActivityEvent[] }> => {
    const params = new URLSearchParams()
    params.set('limit', String(opts?.limit ?? 50))
    if (opts?.subjectType) params.set('subject_type', opts.subjectType)
    if (opts?.action) params.set('action', opts.action)
    return req(`/activity?${params.toString()}`, {}, activityEventsEnvelopeSchema)
  },

  listNotifications: (limit = 50): Promise<{ notifications: Notification[] }> =>
    req(`/notifications?limit=${limit}`, {}, notificationsEnvelopeSchema),

  unreadNotificationCount: (): Promise<{ count: number }> => req('/notifications/unread-count', {}, unreadCountSchema),

  markNotificationRead: (id: string): Promise<{ notification: Notification }> =>
    req(`/notifications/${id}/read`, { method: 'POST' }, notificationEnvelopeSchema),

  markAllNotificationsRead: (): Promise<{ ok: boolean; updated?: number }> =>
    req('/notifications/read-all', { method: 'POST' }, markAllReadSchema),

  shareInvites: (): Promise<{ invites: ShareInvite[] }> => req('/share-invites', {}, shareInvitesEnvelopeSchema),

  acceptShareInvite: (id: string): Promise<unknown> => req(`/share-invites/${id}/accept`, { method: 'POST' }),

  rejectShareInvite: (id: string): Promise<unknown> => req(`/share-invites/${id}/reject`, { method: 'POST' }),

  restoreShareInvite: (id: string): Promise<unknown> => req(`/share-invites/${id}/restore`, { method: 'POST' }),

  getSocial: (subjectType: SocialSubjectType, subjectId: string): Promise<{ social: SocialSummary }> =>
    req(`/social/${subjectType}/${subjectId}`, {}, socialEnvelopeSchema),

  likeSocial: (subjectType: SocialSubjectType, subjectId: string): Promise<{ social: SocialSummary }> =>
    req(`/social/${subjectType}/${subjectId}/like`, { method: 'POST' }, socialEnvelopeSchema),

  unlikeSocial: (subjectType: SocialSubjectType, subjectId: string): Promise<{ social: SocialSummary }> =>
    req(`/social/${subjectType}/${subjectId}/like`, { method: 'DELETE' }, socialEnvelopeSchema),

  commentSocial: (
    subjectType: SocialSubjectType,
    subjectId: string,
    body: string
  ): Promise<{ social: SocialSummary }> =>
    req(
      `/social/${subjectType}/${subjectId}/comments`,
      { method: 'POST', body: JSON.stringify({ body }) },
      socialEnvelopeSchema
    ),

  deleteSocialComment: (id: string): Promise<{ ok: boolean }> =>
    req(`/social/comments/${id}`, { method: 'DELETE' }, okSchema),

  rateSocial: (subjectType: SocialSubjectType, subjectId: string, rating: number): Promise<{ social: SocialSummary }> =>
    req(
      `/social/${subjectType}/${subjectId}/rating`,
      { method: 'POST', body: JSON.stringify({ rating }) },
      socialEnvelopeSchema
    ),

  clearSocialRating: (subjectType: SocialSubjectType, subjectId: string): Promise<{ social: SocialSummary }> =>
    req(`/social/${subjectType}/${subjectId}/rating`, { method: 'DELETE' }, socialEnvelopeSchema),

  register: (body: {
    email: string
    password: string
    display_name?: string
    invite_code?: string
  }): Promise<{ user: UserPublic; token: string }> =>
    req('/auth/register', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  login: (body: { email: string; password: string }): Promise<{ user: UserPublic; token: string }> =>
    req('/auth/login', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  logout: (): Promise<{ ok: boolean }> => req('/auth/logout', { method: 'POST' }, okSchema),

  me: (): Promise<{
    user: UserPublic
    settings: UserSettings
    vault_unlocked: boolean
    api_keys_allowed: boolean
  }> => req('/auth/me', {}, meResponseSchema),

  completeOnboarding: (): Promise<{ user: UserPublic }> =>
    req('/auth/onboarding/complete', { method: 'POST' }, userEnvelopeSchema),

  updateProfile: (body: {
    display_name?: string | null
    bio?: string | null
    timezone?: string | null
    email?: string
    profile_visibility?: ProfileVisibility
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

  myFollowers: (): Promise<{ users: FollowUser[] }> => req('/users/me/followers', {}, followUsersEnvelopeSchema),

  myFollowing: (): Promise<{ users: FollowUser[] }> => req('/users/me/following', {}, followUsersEnvelopeSchema),

  followUser: (id: string): Promise<{ follow: FollowRelation }> =>
    req(`/users/${id}/follow`, { method: 'POST' }, followEnvelopeSchema),

  unfollowUser: (id: string): Promise<{ follow: FollowRelation }> =>
    req(`/users/${id}/follow`, { method: 'DELETE' }, followEnvelopeSchema),

  userProfile: (id: string): Promise<Profile> => req(`/users/${id}/profile`, {}, profileSchema),

  userBookmarks: (id: string, limit: number, offset: number): Promise<ProfileBookmarksPage> =>
    req(`/users/${id}/bookmarks?limit=${limit}&offset=${offset}`, {}, profileBookmarksPageSchema),

  userFollowers: (id: string): Promise<{ users: ProfileSummary[] }> =>
    req(`/users/${id}/followers`, {}, profileSummariesEnvelopeSchema),

  userFollowing: (id: string): Promise<{ users: ProfileSummary[] }> =>
    req(`/users/${id}/following`, {}, profileSummariesEnvelopeSchema),

  followStatus: (id: string): Promise<{ follow: FollowRelation }> =>
    req(`/users/${id}/follow-status`, {}, followEnvelopeSchema),

  myBlocked: (): Promise<{ users: FollowUser[] }> => req('/users/me/blocked', {}, followUsersEnvelopeSchema),

  myMuted: (): Promise<{ users: FollowUser[] }> => req('/users/me/muted', {}, followUsersEnvelopeSchema),

  blockUser: (id: string): Promise<{ block: BlockRelation }> =>
    req(`/users/${id}/block`, { method: 'POST' }, blockEnvelopeSchema),

  unblockUser: (id: string): Promise<{ block: BlockRelation }> =>
    req(`/users/${id}/block`, { method: 'DELETE' }, blockEnvelopeSchema),

  muteUser: (id: string): Promise<{ mute: MuteRelation }> =>
    req(`/users/${id}/mute`, { method: 'POST' }, muteEnvelopeSchema),

  unmuteUser: (id: string): Promise<{ mute: MuteRelation }> =>
    req(`/users/${id}/mute`, { method: 'DELETE' }, muteEnvelopeSchema),

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
      icon: string | null
      visibility: CategoryVisibility
    }>
  ): Promise<Category> => req(`/categories/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, categorySchema),

  deleteCategory: (id: string): Promise<void> => req(`/categories/${id}`, { method: 'DELETE' }),

  forkCategory: (id: string, body: { name?: string } = {}): Promise<ForkCategoryResult> =>
    req(`/categories/${id}/fork`, { method: 'POST', body: JSON.stringify(body) }, forkCategoryResultSchema),

  categoryShares: (categoryId: string): Promise<CategorySharesEnvelope> =>
    req(`/categories/${categoryId}/shares`, {}, categorySharesEnvelopeSchema),

  createCategoryShare: (
    categoryId: string,
    body: { email: string; access: ShareAccess }
  ): Promise<CreateCategoryShareOutcome> =>
    req(
      `/categories/${categoryId}/shares`,
      { method: 'POST', body: JSON.stringify(body) },
      createCategoryShareOutcomeSchema
    ),

  updateCategoryShare: (categoryId: string, userId: string, body: { access: ShareAccess }): Promise<CategoryShare> =>
    req(
      `/categories/${categoryId}/shares/${userId}`,
      { method: 'PATCH', body: JSON.stringify(body) },
      categoryShareSchema
    ),

  deleteCategoryShare: (categoryId: string, userId: string): Promise<void> =>
    req(`/categories/${categoryId}/shares/${userId}`, { method: 'DELETE' }),

  updateCategoryPendingShare: (
    categoryId: string,
    pendingId: string,
    body: { access: ShareAccess }
  ): Promise<PendingShareInvite> =>
    req(
      `/categories/${categoryId}/pending-shares/${pendingId}`,
      { method: 'PATCH', body: JSON.stringify(body) },
      pendingShareInviteSchema
    ),

  deleteCategoryPendingShare: (categoryId: string, pendingId: string): Promise<void> =>
    req(`/categories/${categoryId}/pending-shares/${pendingId}`, { method: 'DELETE' }),

  categoryPublicLink: (categoryId: string): Promise<CategoryPublicLink> =>
    req(`/categories/${categoryId}/public-link`, {}, categoryPublicLinkSchema),

  enableCategoryPublicLink: (categoryId: string): Promise<CategoryPublicLink> =>
    req(`/categories/${categoryId}/public-link`, { method: 'POST' }, categoryPublicLinkSchema),

  rotateCategoryPublicLink: (categoryId: string): Promise<CategoryPublicLink> =>
    req(`/categories/${categoryId}/public-link/rotate`, { method: 'POST' }, categoryPublicLinkSchema),

  disableCategoryPublicLink: (categoryId: string): Promise<CategoryPublicLink> =>
    req(`/categories/${categoryId}/public-link`, { method: 'DELETE' }, categoryPublicLinkSchema),

  publicList: (slug: string): Promise<PublicList> =>
    req(`/public/lists/${encodeURIComponent(slug)}`, {}, publicListSchema),

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
    vault_id?: string | null
    category_ids?: string[]
    tags: string[]
    is_favorite?: boolean
    is_unread?: boolean
    is_pinned?: boolean
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
      is_favorite: boolean
      is_unread: boolean
      is_pinned: boolean
      is_private: boolean
    }>
  ): Promise<Bookmark> => req(`/bookmarks/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, bookmarkSchema),

  deleteBookmark: (id: string): Promise<void> => req(`/bookmarks/${id}`, { method: 'DELETE' }),

  uploadBookmarkPreview: async (id: string, blob: Blob): Promise<Bookmark> => {
    const fd = new FormData()
    fd.append('preview', blob, 'preview.jpg')
    return req(`/bookmarks/${id}/preview`, { method: 'POST', body: fd }, bookmarkSchema)
  },

  deleteBookmarkPreview: (id: string): Promise<Bookmark> =>
    req(`/bookmarks/${id}/preview`, { method: 'DELETE' }, bookmarkSchema),

  search: (q: string) => req(`/search?q=${encodeURIComponent(q)}`, {}, searchResponseSchema),

  import: (format: string, content: string): Promise<{ total: number; imported: number; failed: number }> =>
    req('/import', { method: 'POST', body: JSON.stringify({ format, content }) }, importResultSchema),

  exportUrl: (format: string): string => `/api/export?format=${format}`,

  vaults: (): Promise<{ vaults: Vault[] }> => req('/vaults', {}, vaultsEnvelopeSchema),

  createVault: (body: { name: string; passphrase: string }): Promise<{ vault: Vault; recovery_key: string }> =>
    req('/vaults', { method: 'POST', body: JSON.stringify(body) }, createdVaultSchema),

  renameVault: (id: string, name: string): Promise<unknown> =>
    req(`/vaults/${id}`, { method: 'PATCH', body: JSON.stringify({ name }) }),

  deleteVault: (id: string, body: { name: string; password?: string }): Promise<unknown> =>
    req(`/vaults/${id}`, { method: 'DELETE', body: JSON.stringify(body) }),

  unlockVault: (id: string, passphrase: string): Promise<unknown> =>
    req(`/vaults/${id}/unlock`, { method: 'POST', body: JSON.stringify({ passphrase }) }),

  lockVault: (id: string): Promise<unknown> => req(`/vaults/${id}/lock`, { method: 'POST' }),

  lockAllVaults: (): Promise<unknown> => req('/vaults/lock-all', { method: 'POST' }),

  recoverVault: (id: string, body: { recovery_key: string; new_passphrase: string }): Promise<unknown> =>
    req(`/vaults/${id}/recover`, { method: 'POST', body: JSON.stringify(body) }),

  newRecoveryKey: (id: string): Promise<{ recovery_key: string }> =>
    req(`/vaults/${id}/recovery-key`, { method: 'POST' }, recoveryKeySchema),

  globalPassphrase: (): Promise<{ enabled: boolean }> => req('/vaults/global', {}, globalPassphraseSchema),

  enableGlobalPassphrase: (passphrase: string): Promise<unknown> =>
    req('/vaults/global', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  changeGlobalPassphrase: (body: { current_passphrase: string; new_passphrase: string }): Promise<unknown> =>
    req('/vaults/global', { method: 'PUT', body: JSON.stringify(body) }),

  disableGlobalPassphrase: (): Promise<unknown> => req('/vaults/global', { method: 'DELETE' }),

  unlockAllVaults: (passphrase: string): Promise<unknown> =>
    req('/vaults/global/unlock', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  joinGlobal: (id: string, passphrase: string): Promise<unknown> =>
    req(`/vaults/${id}/global`, { method: 'POST', body: JSON.stringify({ passphrase }) }),

  leaveGlobal: (id: string): Promise<unknown> => req(`/vaults/${id}/global`, { method: 'DELETE' }),

  moveToVault: (body: {
    category_ids: string[]
    bookmark_ids: string[]
    destination_vault_id: string | null
  }): Promise<unknown> => req('/vaults/move', { method: 'POST', body: JSON.stringify(body) }),

  duplicates: (): Promise<{ urls: string[]; groups: DuplicateGroup[] }> =>
    req('/bookmarks/duplicates', {}, duplicatesSchema),

  brokenBookmarks: (): Promise<Bookmark[]> => req('/bookmarks/broken-links', {}, z.array(bookmarkSchema)),

  checkBookmarkLink: (id: string): Promise<Bookmark> =>
    req(`/bookmarks/${id}/check-link`, { method: 'POST' }, bookmarkSchema),

  saveBookmarkCopy: (id: string): Promise<Bookmark> => req(`/bookmarks/${id}/save`, { method: 'POST' }, bookmarkSchema),

  listDashboards: (): Promise<DashboardSummary[]> => req('/dashboards', {}, dashboardSummariesSchema),

  createDashboard: (body: CreateDashboard = {}): Promise<Dashboard> =>
    req('/dashboards', { method: 'POST', body: JSON.stringify(body) }, dashboardSchema),

  getDefaultDashboard: (): Promise<Dashboard> => req('/dashboards/default', {}, dashboardSchema),

  putDefaultDashboard: (body: PutDashboardLayout): Promise<Dashboard> =>
    req('/dashboards/default', { method: 'PUT', body: JSON.stringify(body) }, dashboardSchema),

  getDashboard: (id: string): Promise<Dashboard> => req(`/dashboards/${id}`, {}, dashboardSchema),

  putDashboard: (id: string, body: PutDashboardLayout): Promise<Dashboard> =>
    req(`/dashboards/${id}`, { method: 'PUT', body: JSON.stringify(body) }, dashboardSchema),

  patchDashboard: (id: string, body: PatchDashboard): Promise<Dashboard> =>
    req(`/dashboards/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, dashboardSchema),

  deleteDashboard: (id: string): Promise<DashboardSummary[]> =>
    req(`/dashboards/${id}`, { method: 'DELETE' }, dashboardSummariesSchema),

  dashboardShares: (dashboardId: string): Promise<DashboardSharesEnvelope> =>
    req(`/dashboards/${dashboardId}/shares`, {}, dashboardSharesEnvelopeSchema),

  createDashboardShare: (
    dashboardId: string,
    body: { email: string; access: ShareAccess }
  ): Promise<CreateDashboardShareOutcome> =>
    req(
      `/dashboards/${dashboardId}/shares`,
      { method: 'POST', body: JSON.stringify(body) },
      createDashboardShareOutcomeSchema
    ),

  updateDashboardShare: (dashboardId: string, userId: string, body: { access: ShareAccess }): Promise<DashboardShare> =>
    req(
      `/dashboards/${dashboardId}/shares/${userId}`,
      { method: 'PATCH', body: JSON.stringify(body) },
      dashboardShareSchema
    ),

  deleteDashboardShare: (dashboardId: string, userId: string): Promise<void> =>
    req(`/dashboards/${dashboardId}/shares/${userId}`, { method: 'DELETE' }),

  updateDashboardPendingShare: (
    dashboardId: string,
    pendingId: string,
    body: { access: ShareAccess }
  ): Promise<PendingShareInvite> =>
    req(
      `/dashboards/${dashboardId}/pending-shares/${pendingId}`,
      { method: 'PATCH', body: JSON.stringify(body) },
      pendingShareInviteSchema
    ),

  deleteDashboardPendingShare: (dashboardId: string, pendingId: string): Promise<void> =>
    req(`/dashboards/${dashboardId}/pending-shares/${pendingId}`, { method: 'DELETE' }),

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
