import { req } from './client'
import {
  authTokenResponseSchema,
  bootstrapStatusSchema,
  meResponseSchema,
  okSchema,
  settingsEnvelopeSchema,
  userEnvelopeSchema,
  type UserPublic,
  type UserSettings,
} from '../schemas/auth'
import {
  bookmarkSchema,
  duplicatesSchema,
  importResultSchema,
  searchResponseSchema,
  type Bookmark,
} from '../schemas/bookmarks'
import { categoriesSchema, categorySchema, type Category } from '../schemas/categories'
import { vaultStatusSchema } from '../schemas/vault'
import { z } from 'zod'

const bookmarksSchema = z.array(bookmarkSchema)

export const api = {
  bootstrap: (): Promise<{ needs_bootstrap: boolean; oidc_enabled: boolean }> =>
    req('/auth/bootstrap-status', {}, bootstrapStatusSchema),

  register: (body: {
    email: string
    password: string
    display_name?: string
  }): Promise<{ user: UserPublic; token: string }> =>
    req('/auth/register', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  login: (body: {
    email: string
    password: string
  }): Promise<{ user: UserPublic; token: string }> =>
    req('/auth/login', { method: 'POST', body: JSON.stringify(body) }, authTokenResponseSchema),

  logout: (): Promise<{ ok: boolean }> =>
    req('/auth/logout', { method: 'POST' }, okSchema),

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

  changePassword: (body: {
    current_password: string
    new_password: string
  }): Promise<{ ok: boolean }> =>
    req('/users/me/password', { method: 'POST', body: JSON.stringify(body) }, okSchema),

  getSettings: (): Promise<{ settings: UserSettings }> =>
    req('/users/me/settings', {}, settingsEnvelopeSchema),

  updateSettings: (
    body: Partial<{
      show_hidden: boolean
      vault_search_mode: UserSettings['vault_search_mode']
      theme: UserSettings['theme']
      vault_auto_lock_minutes: number
      default_landing: UserSettings['default_landing']
    }>,
  ): Promise<{ settings: UserSettings }> =>
    req(
      '/users/me/settings',
      { method: 'PATCH', body: JSON.stringify(body) },
      settingsEnvelopeSchema,
    ),

  uploadAvatar: async (blob: Blob): Promise<{ user: UserPublic }> => {
    const fd = new FormData()
    fd.append('avatar', blob, 'avatar.jpg')
    return req('/users/me/avatar', { method: 'POST', body: fd }, userEnvelopeSchema)
  },

  deleteAvatar: (): Promise<{ user: UserPublic }> =>
    req('/users/me/avatar', { method: 'DELETE' }, userEnvelopeSchema),

  categories: (include_hidden = false, include_vault = false): Promise<Category[]> =>
    req(
      `/categories?include_hidden=${include_hidden}&include_vault=${include_vault}`,
      {},
      categoriesSchema,
    ),

  createCategory: (body: Partial<Category> & { name: string }): Promise<Category> =>
    req('/categories', { method: 'POST', body: JSON.stringify(body) }, categorySchema),

  updateCategory: (
    id: string,
    body: Partial<{
      name: string
      parent_id: string | null
      is_hidden: boolean
      position: number
    }>,
  ): Promise<Category> =>
    req(`/categories/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, categorySchema),

  deleteCategory: (id: string): Promise<void> =>
    req(`/categories/${id}`, { method: 'DELETE' }),

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
  }): Promise<Bookmark> =>
    req('/bookmarks', { method: 'POST', body: JSON.stringify(body) }, bookmarkSchema),

  updateBookmark: (
    id: string,
    body: Partial<{
      url: string
      title: string
      description: string | null
      notes: string | null
      category_ids: string[]
      tags: string[]
    }>,
  ): Promise<Bookmark> =>
    req(`/bookmarks/${id}`, { method: 'PATCH', body: JSON.stringify(body) }, bookmarkSchema),

  deleteBookmark: (id: string): Promise<void> =>
    req(`/bookmarks/${id}`, { method: 'DELETE' }),

  search: (q: string) =>
    req(`/search?q=${encodeURIComponent(q)}`, {}, searchResponseSchema),

  import: (format: string, content: string): Promise<{ total: number; imported: number; failed: number }> =>
    req('/import', { method: 'POST', body: JSON.stringify({ format, content }) }, importResultSchema),

  exportUrl: (format: string): string => `/api/export?format=${format}`,

  vaultStatus: (): Promise<{ configured: boolean; unlocked: boolean }> =>
    req('/vault/status', {}, vaultStatusSchema),

  vaultSetup: (passphrase: string): Promise<unknown> =>
    req('/vault/setup', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  vaultUnlock: (passphrase: string): Promise<unknown> =>
    req('/vault/unlock', { method: 'POST', body: JSON.stringify({ passphrase }) }),

  vaultLock: (): Promise<unknown> => req('/vault/lock', { method: 'POST' }),

  duplicates: (): Promise<{ urls: string[] }> =>
    req('/bookmarks/duplicates', {}, duplicatesSchema),
}
