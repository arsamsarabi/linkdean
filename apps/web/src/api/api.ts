import { z } from 'zod'
import type { SortKey } from '@/lib'
import {
  abuseReportEnvelopeSchema,
  abuseReportsEnvelopeSchema,
  activityEventsEnvelopeSchema,
  adminEmailChangeResponseSchema,
  adminEmailVerifiedResponseSchema,
  adminReindexSchema,
  adminResetLinkSchema,
  adminUserEnvelopeSchema,
  adminUsersEnvelopeSchema,
  apiKeysEnvelopeSchema,
  authTokenResponseSchema,
  bansEnvelopeSchema,
  blockEnvelopeSchema,
  bookmarkIdsSchema,
  bookmarkListPageSchema,
  bookmarkLookupSchema,
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
  emailChangeResponseSchema,
  emailStatusSchema,
  exposedContentEnvelopeSchema,
  extensionAuthorizeSchema,
  filingSuggestionsSchema,
  followEnvelopeSchema,
  followUsersEnvelopeSchema,
  forgotPasswordResponseSchema,
  forkCategoryResultSchema,
  globalPassphraseSchema,
  importPreviewSchema,
  importResultSchema,
  markAllReadSchema,
  meResponseSchema,
  moderateOutcomeEnvelopeSchema,
  moderationActionsEnvelopeSchema,
  muteEnvelopeSchema,
  notificationEmailPrefSchema,
  notificationEmailPrefsEnvelopeSchema,
  notificationEnvelopeSchema,
  notificationsEnvelopeSchema,
  okSchema,
  okUserEnvelopeSchema,
  pageDetailsSchema,
  pendingOidcLinkSchema,
  pendingShareInviteSchema,
  platformSettingsEnvelopeSchema,
  profileBookmarksPageSchema,
  profileSchema,
  profileSummariesEnvelopeSchema,
  publicListSchema,
  quotaUsageSchema,
  recoveryKeySchema,
  registrationInvitesEnvelopeSchema,
  reportConfigSchema,
  reportFiledSchema,
  resendVerificationResponseSchema,
  scheduleDeletionResponseSchema,
  screenshotRetryStatusSchema,
  searchResponseSchema,
  settingsEnvelopeSchema,
  shareInvitesEnvelopeSchema,
  socialEnvelopeSchema,
  tagNamesSchema,
  testEmailResultSchema,
  trashItemsSchema,
  trashPurgeResultSchema,
  unreadCountSchema,
  userEnvelopeSchema,
  vaultsEnvelopeSchema,
  type AbuseReport,
  type ActivityAction,
  type ActivityEvent,
  type ActivitySubjectType,
  type AdminEmailChangeResponse,
  type AdminEmailVerifiedResponse,
  type AdminResetLink,
  type AdminUserSummary,
  type ApiKey,
  type AuthTokenResponse,
  type BanDurationBody,
  type BanView,
  type BlockRelation,
  type Bookmark,
  type BookmarkListPage,
  type BootstrapStatus,
  type Category,
  type CategoryPublicLink,
  type CategoryShare,
  type CategorySharesEnvelope,
  type CategoryVisibility,
  type CreateApiKeyResponse,
  type CreateCategoryBody,
  type CreateCategoryShareOutcome,
  type CreateDashboard,
  type CreateDashboardShareOutcome,
  type CreateRegistrationInviteResponse,
  type Dashboard,
  type DashboardShare,
  type DashboardSharesEnvelope,
  type DashboardSummary,
  type DuplicateDecisions,
  type DuplicateGroup,
  type EmailChangeResponse,
  type EmailStatus,
  type ExposedContent,
  type FilingSuggestions,
  type FollowRelation,
  type FollowUser,
  type ForgotPasswordResponse,
  type ForkCategoryResult,
  type ImportPreview,
  type ImportResult,
  type ModerateItem,
  type ModerateOutcome,
  type ModerationAction,
  type MuteRelation,
  type NewAbuseReport,
  type Notification,
  type NotificationEmailPref,
  type NotificationKind,
  type PatchDashboard,
  type PendingOidcLink,
  type PendingShareInvite,
  type PlatformSettings,
  type Profile,
  type ProfileBookmarksPage,
  type ProfileSummary,
  type ProfileVisibility,
  type PublicList,
  type PutDashboardLayout,
  type QuotaUsage,
  type RegistrationInvite,
  type ReportConfig,
  type ResendVerificationResponse,
  type ResolveAbuseReport,
  type ScheduleDeletionResponse,
  type ScreenshotRetryStatus,
  type ShareAccess,
  type ShareInvite,
  type SocialSubjectType,
  type SocialSummary,
  type TestEmailResult,
  type TrashItem,
  type TrashKind,
  type UserPublic,
  type UserSettings,
  type Vault,
} from '@/schemas'
import { req } from './client'

/** Filters shared by `/bookmarks` and `/bookmarks/ids`. `folder`: category uuid, `uncategorized`, or omit for all. */
export type BookmarksListParams = {
  folder?: string | null
  limit?: number
  cursor?: string | null
  sort?: SortKey
  tag?: readonly string[]
  flag?: readonly string[]
  since?: string | null
  include_vault?: boolean
  vault_only?: boolean
}

function bookmarksQuery(p: BookmarksListParams): URLSearchParams {
  const q = new URLSearchParams()
  if (p.folder) q.set('folder', p.folder)
  if (p.limit) q.set('limit', String(p.limit))
  if (p.cursor) q.set('cursor', p.cursor)
  if (p.sort) q.set('sort', p.sort)
  if (p.tag?.length) q.set('tag', p.tag.join(','))
  if (p.flag?.length) q.set('flag', p.flag.join(','))
  if (p.since) q.set('since', p.since)
  if (p.include_vault) q.set('include_vault', 'true')
  if (p.vault_only) q.set('vault_only', 'true')
  return q
}

export const api = {
  bootstrap: (): Promise<BootstrapStatus> => req('/auth/bootstrap-status', {}, bootstrapStatusSchema),

  getPlatformSettings: (): Promise<{
    settings: PlatformSettings
    require_verified_email_forced: boolean
  }> => req('/admin/platform-settings', {}, platformSettingsEnvelopeSchema),

  updatePlatformSettings: (body: {
    registration_open?: boolean
    api_keys_enabled?: boolean
    require_verified_email?: boolean
    max_bookmarks?: number | null
    max_stored_image_bytes?: number | null
    max_enrich_actions_per_day?: number | null
    max_import_bytes?: number | null
    max_upload_image_bytes?: number | null
    abuse_contact_email?: string | null
    screenshot_retry_cooldown_hours?: number
    screenshot_auto_retry_days?: number
  }): Promise<{
    settings: PlatformSettings
    require_verified_email_forced: boolean
  }> =>
    req('/admin/platform-settings', { method: 'PATCH', body: JSON.stringify(body) }, platformSettingsEnvelopeSchema),

  getQuotaUsage: (): Promise<QuotaUsage> => req('/users/me/quota-usage', {}, quotaUsageSchema),

  listAdminUsers: (): Promise<{ users: AdminUserSummary[] }> => req('/admin/users', {}, adminUsersEnvelopeSchema),

  updateAdminUser: (
    id: string,
    body: { api_keys_enabled?: boolean; is_admin?: boolean }
  ): Promise<{ user: AdminUserSummary }> =>
    req(`/admin/users/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, adminUserEnvelopeSchema),

  createResetLink: (id: string): Promise<AdminResetLink> =>
    req(`/admin/users/${id}/reset-link`, { method: 'POST' }, adminResetLinkSchema),

  setEmailVerified: (id: string, verified: boolean): Promise<AdminEmailVerifiedResponse> =>
    req(
      `/admin/users/${id}/email-verified`,
      { method: 'POST', body: JSON.stringify({ verified }) },
      adminEmailVerifiedResponseSchema
    ),

  /** Applies at once and marks the address verified: the admin vouches for it. */
  adminChangeEmail: (id: string, email: string): Promise<AdminEmailChangeResponse> =>
    req(
      `/admin/users/${id}/email`,
      { method: 'POST', body: JSON.stringify({ email }) },
      adminEmailChangeResponseSchema
    ),

  listBans: (): Promise<{ bans: BanView[] }> => req('/admin/bans', {}, bansEnvelopeSchema),

  banUser: (id: string, body: { reason: string } & BanDurationBody): Promise<unknown> =>
    req(`/admin/users/${id}/ban`, { method: 'POST', body: JSON.stringify(body) }),

  changeBans: (body: { user_ids: string[]; reason?: string } & BanDurationBody): Promise<{ ok: boolean }> =>
    req('/admin/bans', { method: 'PATCH', body: JSON.stringify(body) }, okSchema),

  liftBans: (user_ids: string[]): Promise<{ ok: boolean }> =>
    req('/admin/bans/lift', { method: 'POST', body: JSON.stringify({ user_ids }) }, okSchema),

  exposedContent: (id: string): Promise<{ content: ExposedContent }> =>
    req(`/admin/users/${id}/exposed-content`, {}, exposedContentEnvelopeSchema),

  moderate: (id: string, body: { reason: string; actions: ModerateItem[] }): Promise<{ outcome: ModerateOutcome }> =>
    req(`/admin/users/${id}/moderate`, { method: 'POST', body: JSON.stringify(body) }, moderateOutcomeEnvelopeSchema),

  moderationActions: (id: string): Promise<{ actions: ModerationAction[] }> =>
    req(`/admin/users/${id}/moderation-actions`, {}, moderationActionsEnvelopeSchema),

  createReport: (body: NewAbuseReport): Promise<{ ok: boolean; id: string }> =>
    req('/reports', { method: 'POST', body: JSON.stringify(body) }, reportFiledSchema),

  reportConfig: (): Promise<ReportConfig> => req('/reports/config', {}, reportConfigSchema),

  listAbuseReports: (status: 'open' | 'resolved'): Promise<{ reports: AbuseReport[] }> =>
    req(`/admin/reports?status=${status}`, {}, abuseReportsEnvelopeSchema),

  resolveAbuseReport: (id: string, body: ResolveAbuseReport): Promise<{ report: AbuseReport }> =>
    req(`/admin/reports/${id}/resolve`, { method: 'POST', body: JSON.stringify(body) }, abuseReportEnvelopeSchema),

  screenshotRetryStatus: (): Promise<ScreenshotRetryStatus> =>
    req('/admin/screenshots/retry', {}, screenshotRetryStatusSchema),

  retryScreenshots: (): Promise<ScreenshotRetryStatus> =>
    req('/admin/screenshots/retry', { method: 'POST' }, screenshotRetryStatusSchema),

  reindexSearch: (): Promise<{ ok: boolean; upserted: number; failed: number }> =>
    req('/admin/search/reindex', { method: 'POST' }, adminReindexSchema),

  getEmailStatus: (): Promise<EmailStatus> => req('/admin/email', {}, emailStatusSchema),

  sendTestEmail: (): Promise<TestEmailResult> => req('/admin/email/test', { method: 'POST' }, testEmailResultSchema),

  listRegistrationInvites: (): Promise<{ invites: RegistrationInvite[] }> =>
    req('/admin/invites', {}, registrationInvitesEnvelopeSchema),

  createRegistrationInvite: (body: {
    email?: string
    expires_in_days?: number
  }): Promise<CreateRegistrationInviteResponse> =>
    req('/admin/invites', { method: 'POST', body: JSON.stringify(body) }, createRegistrationInviteResponseSchema),

  revokeRegistrationInvite: (id: string): Promise<{ ok: boolean }> =>
    req(`/admin/invites/${id}`, { method: 'DELETE' }, okSchema),

  /** Approves the browser extension's sign-in; the reply is the code to hand to it. */
  authorizeExtension: (body: {
    extension_origin: string
    code_challenge: string
    code_challenge_method: string
    state?: string
  }) => req('/auth/extension/authorize', { method: 'POST', body: JSON.stringify(body) }, extensionAuthorizeSchema),

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

  notificationEmailPrefs: (): Promise<{ prefs: NotificationEmailPref[] }> =>
    req('/users/me/notification-emails', {}, notificationEmailPrefsEnvelopeSchema),

  setNotificationEmailPref: (kind: NotificationKind, enabled: boolean): Promise<NotificationEmailPref> =>
    req(
      `/users/me/notification-emails/${kind}`,
      { method: 'PUT', body: JSON.stringify({ enabled }) },
      notificationEmailPrefSchema
    ),

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
    bootstrap_token?: string
    share_token?: string
    turnstile_token?: string
    terms_version: string
  }): Promise<{ user: UserPublic; token: string }> =>
    req('/auth/register', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  login: (body: { email: string; password: string }): Promise<AuthTokenResponse> =>
    req('/auth/login', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  /** The single sign-on this browser holds waiting for a password sign-in; 404 when there is none. */
  pendingOidcLink: (): Promise<PendingOidcLink> => req('/auth/oidc/pending-link', {}, pendingOidcLinkSchema),

  /** Detaches single sign-on; needs the account's password. A wrong one is a 401 that keeps the session. */
  disconnectSso: (password: string): Promise<{ ok: boolean; user: UserPublic }> =>
    req('/auth/oidc/disconnect', { method: 'POST', body: JSON.stringify({ password }) }, okUserEnvelopeSchema),

  forgotPassword: (email: string): Promise<ForgotPasswordResponse> =>
    req('/auth/password/forgot', { method: 'POST', body: JSON.stringify({ email }) }, forgotPasswordResponseSchema),

  resetPassword: (body: {
    token: string
    password: string
    revoke_api_keys: boolean
  }): Promise<{ user: UserPublic; token: string }> =>
    req('/auth/password/reset', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  resendVerification: (): Promise<ResendVerificationResponse> =>
    req('/auth/email/resend', { method: 'POST' }, resendVerificationResponseSchema),

  verifyEmail: (token: string): Promise<{ ok: boolean }> =>
    req('/auth/email/verify', { method: 'POST', body: JSON.stringify({ token }) }, okSchema),

  /** Sends a link to the new address; the email stays the same until it is opened. */
  changeEmail: (body: { password: string; new_email: string }): Promise<EmailChangeResponse> =>
    req('/auth/email/change', { method: 'POST', body: JSON.stringify(body) }, emailChangeResponseSchema),

  confirmEmailChange: (token: string): Promise<{ ok: boolean }> =>
    req('/auth/email/confirm-change', { method: 'POST', body: JSON.stringify({ token }) }, okSchema),

  /** Claims an invite link sent to this account's own address while signed in. */
  claimShareInvite: (token: string): Promise<{ ok: boolean }> =>
    req('/shares/claim', { method: 'POST', body: JSON.stringify({ token }) }, okSchema),

  logout: (): Promise<{ ok: boolean }> => req('/auth/logout', { method: 'POST' }, okSchema),

  me: (): Promise<{
    user: UserPublic
    settings: UserSettings
    vault_unlocked: boolean
    api_keys_allowed: boolean
    require_verified_email: boolean
  }> => req('/auth/me', {}, meResponseSchema),

  completeOnboarding: (): Promise<{ user: UserPublic }> =>
    req('/auth/onboarding/complete', { method: 'POST' }, userEnvelopeSchema),

  updateProfile: (body: {
    display_name?: string | null
    bio?: string | null
    timezone?: string | null
    profile_visibility?: ProfileVisibility
  }): Promise<{ user: UserPublic }> =>
    req('/users/me', { method: 'PATCH', body: JSON.stringify(body) }, userEnvelopeSchema),

  changePassword: (body: {
    current_password: string
    new_password: string
    revoke_api_keys?: boolean
  }): Promise<{ ok: boolean }> => req('/users/me/password', { method: 'POST', body: JSON.stringify(body) }, okSchema),

  /** Starts the 7-day grace period. Password accounts must send their password. */
  scheduleAccountDeletion: (body: { confirm_email: string; password?: string }): Promise<ScheduleDeletionResponse> =>
    req('/users/me/deletion', { method: 'POST', body: JSON.stringify(body) }, scheduleDeletionResponseSchema),

  cancelAccountDeletion: (): Promise<{ ok: boolean }> => req('/users/me/deletion', { method: 'DELETE' }, okSchema),

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

  createCategory: (body: CreateCategoryBody): Promise<Category> =>
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

  bookmarksPage: (params: BookmarksListParams): Promise<BookmarkListPage> =>
    req(`/bookmarks?${bookmarksQuery(params)}`, {}, bookmarkListPageSchema),

  bookmark: (id: string): Promise<Bookmark> => req(`/bookmarks/${id}`, {}, bookmarkSchema),

  /** Folders and tags the person already uses on this bookmark's site. */
  filingSuggestions: (id: string): Promise<FilingSuggestions> =>
    req(`/bookmarks/${id}/filing-suggestions`, {}, filingSuggestionsSchema),

  /** Ids matching the same filters as `bookmarksPage` (server caps at 10k). */
  bookmarkIds: async (params: BookmarksListParams): Promise<string[]> =>
    (await req(`/bookmarks/ids?${bookmarksQuery(params)}`, {}, bookmarkIdsSchema)).ids,

  /** Every bookmark, for dashboard pickers. Walks pages. */
  bookmarks: async (category_id?: string | null, include_vault = false): Promise<Bookmark[]> => {
    const out: Bookmark[] = []
    let cursor: string | undefined
    // ponytail: 10k ceiling (matches /bookmarks/ids); upgrade is a server-searched picker.
    while (out.length < 10_000) {
      const page = await api.bookmarksPage({ folder: category_id, include_vault, limit: 100, cursor })
      out.push(...page.items)
      if (!page.next_cursor) break
      cursor = page.next_cursor
    }
    return out
  },

  tags: async (): Promise<string[]> => (await req('/tags', {}, tagNamesSchema)).map((t) => t.name),

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
      /** Added on the server to the bookmark's current tags. */
      add_tags: string[]
      /** Added on the server to the bookmark's current folders. */
      add_category_ids: string[]
      is_favorite: boolean
      is_unread: boolean
      is_pinned: boolean
      is_private: boolean
    }>
  ): Promise<Bookmark> => req(`/bookmarks/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, bookmarkSchema),

  /** The person's own bookmark at exactly `url`, outside any vault, or null. */
  lookupBookmark: async (url: string): Promise<Bookmark | null> =>
    (await req(`/bookmarks/lookup?url=${encodeURIComponent(url)}`, {}, bookmarkLookupSchema)).bookmark,

  /** Title, description and keyword tags of the page at `url`, fetched by the server. */
  pageDetails: (url: string) =>
    req('/bookmarks/page-details', { method: 'POST', body: JSON.stringify({ url }) }, pageDetailsSchema),

  deleteBookmark: (id: string): Promise<void> => req(`/bookmarks/${id}`, { method: 'DELETE' }),

  uploadBookmarkPreview: async (id: string, blob: Blob): Promise<Bookmark> => {
    const fd = new FormData()
    fd.append('preview', blob, 'preview.jpg')
    return req(`/bookmarks/${id}/preview`, { method: 'POST', body: fd }, bookmarkSchema)
  },

  deleteBookmarkPreview: (id: string): Promise<Bookmark> =>
    req(`/bookmarks/${id}/preview`, { method: 'DELETE' }, bookmarkSchema),

  search: (q: string) => req(`/search?q=${encodeURIComponent(q)}`, {}, searchResponseSchema),

  importPreview: (format: string, content: string): Promise<ImportPreview> =>
    req('/import/preview', { method: 'POST', body: JSON.stringify({ format, content }) }, importPreviewSchema),

  import: (format: string, content: string, duplicates?: DuplicateDecisions): Promise<ImportResult> =>
    req(
      '/import',
      { method: 'POST', body: JSON.stringify(duplicates ? { format, content, duplicates } : { format, content }) },
      importResultSchema
    ),

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

  newRecoveryKey: (id: string, passphrase: string): Promise<{ recovery_key: string }> =>
    req(`/vaults/${id}/recovery-key`, { method: 'POST', body: JSON.stringify({ passphrase }) }, recoveryKeySchema),

  globalPassphrase: (): Promise<{ enabled: boolean }> => req('/vaults/global', {}, globalPassphraseSchema),

  enableGlobalPassphrase: (passphrase: string): Promise<unknown> =>
    req('/vaults/global', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  changeGlobalPassphrase: (body: { current_passphrase: string; new_passphrase: string }): Promise<unknown> =>
    req('/vaults/global', { method: 'PUT', body: JSON.stringify(body) }),

  disableGlobalPassphrase: (passphrase: string): Promise<unknown> =>
    req('/vaults/global', { method: 'DELETE', body: JSON.stringify({ passphrase }) }),

  unlockAllVaults: (passphrase: string): Promise<unknown> =>
    req('/vaults/global/unlock', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  joinGlobal: (id: string): Promise<unknown> => req(`/vaults/${id}/global`, { method: 'POST' }),

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
