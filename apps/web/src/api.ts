export type UserPublic = {
  id: string
  email: string
  display_name?: string | null
  bio?: string | null
  timezone?: string | null
  has_avatar: boolean
  has_password: boolean
  has_vault: boolean
  is_admin: boolean
  onboarding_completed: boolean
}

export type UserSettings = {
  user_id: string
  show_hidden: boolean
  vault_search_mode: 'never' | 'when_unlocked' | 'always'
  theme: 'light' | 'dark' | 'system'
  vault_auto_lock_minutes: number
  default_landing: '/' | '/vault' | '/import'
  updated_at: string
}

export type MeResponse = {
  user: UserPublic
  settings: UserSettings
  vault_unlocked: boolean
}

export type Category = {
  id: string
  parent_id?: string | null
  name: string
  is_hidden: boolean
  is_vault: boolean
  position: number
}

export type CategoryRef = {
  id: string
  name: string
  path: string
}

export type Bookmark = {
  id: string
  category_id?: string | null
  categories: CategoryRef[]
  url: string
  title: string
  description?: string | null
  notes?: string | null
  favicon_url?: string | null
  preview_image_url?: string | null
  has_preview: boolean
  is_vault: boolean
  tags: string[]
  created_at: string
  updated_at: string
}

export type SearchHit = {
  id: string
  title: string
  url: string
  description?: string | null
  favicon_url?: string | null
  preview_image_url?: string | null
  has_preview: boolean
  tags: string[]
  category_id?: string | null
  categories?: CategoryRef[]
  is_vault: boolean
}

/** Local screenshot when present; else remote OG image. */
export function bookmarkPreviewSrc(b: {
  id: string
  has_preview?: boolean
  preview_image_url?: string | null
}) {
  if (b.has_preview) return `/api/bookmarks/${b.id}/preview`
  return b.preview_image_url || undefined
}

export type SearchResponse = {
  hits: SearchHit[]
  vault_prompt: boolean
}

const tokenKey = 'linkdean_token'

export function getToken() {
  return localStorage.getItem(tokenKey)
}

export function setToken(token: string | null) {
  if (token) localStorage.setItem(tokenKey, token)
  else localStorage.removeItem(tokenKey)
}

async function req<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers)
  if (!headers.has('Content-Type') && init.body && !(init.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json')
  }
  const token = getToken()
  if (token) headers.set('Authorization', 'Bearer ' + token)
  const res = await fetch(`/api${path}`, { ...init, headers, credentials: 'include' })
  if (!res.ok) {
    let msg = res.statusText
    try {
      const j = await res.json()
      msg = j.error || msg
    } catch {
      /* ignore */
    }
    throw new Error(msg)
  }
  if (res.status === 204) return undefined as T
  const ct = res.headers.get('content-type') || ''
  if (ct.includes('application/json')) return res.json()
  return (await res.text()) as T
}

export function avatarUrl(hasAvatar: boolean, bust?: number | string) {
  if (!hasAvatar) return undefined
  const q = bust != null ? `?v=${bust}` : `?v=${Date.now()}`
  return `/api/users/me/avatar${q}`
}

export const api = {
  bootstrap: () => req<{ needs_bootstrap: boolean; oidc_enabled: boolean }>('/auth/bootstrap-status'),
  register: (body: { email: string; password: string; display_name?: string }) =>
    req<{ user: UserPublic; token: string }>('/auth/register', { method: 'POST', body: JSON.stringify(body) }),
  login: (body: { email: string; password: string }) =>
    req<{ user: UserPublic; token: string }>('/auth/login', { method: 'POST', body: JSON.stringify(body) }),
  logout: () => req<{ ok: boolean }>('/auth/logout', { method: 'POST' }),
  me: () => req<MeResponse>('/auth/me'),
  completeOnboarding: () =>
    req<{ user: UserPublic }>('/auth/onboarding/complete', { method: 'POST' }),
  updateProfile: (body: {
    display_name?: string | null
    bio?: string | null
    timezone?: string | null
    email?: string
  }) => req<{ user: UserPublic }>('/users/me', { method: 'PATCH', body: JSON.stringify(body) }),
  changePassword: (body: { current_password: string; new_password: string }) =>
    req<{ ok: boolean }>('/users/me/password', { method: 'POST', body: JSON.stringify(body) }),
  getSettings: () => req<{ settings: UserSettings }>('/users/me/settings'),
  updateSettings: (body: Partial<{
    show_hidden: boolean
    vault_search_mode: UserSettings['vault_search_mode']
    theme: UserSettings['theme']
    vault_auto_lock_minutes: number
    default_landing: UserSettings['default_landing']
  }>) => req<{ settings: UserSettings }>('/users/me/settings', { method: 'PATCH', body: JSON.stringify(body) }),
  uploadAvatar: async (blob: Blob) => {
    const fd = new FormData()
    fd.append('avatar', blob, 'avatar.jpg')
    return req<{ user: UserPublic }>('/users/me/avatar', { method: 'POST', body: fd })
  },
  deleteAvatar: () => req<{ user: UserPublic }>('/users/me/avatar', { method: 'DELETE' }),
  categories: (include_hidden = false, include_vault = false) =>
    req<Category[]>(`/categories?include_hidden=${include_hidden}&include_vault=${include_vault}`),
  createCategory: (body: Partial<Category> & { name: string }) =>
    req<Category>('/categories', { method: 'POST', body: JSON.stringify(body) }),
  updateCategory: (
    id: string,
    body: Partial<{
      name: string
      parent_id: string | null
      is_hidden: boolean
      position: number
    }>,
  ) => req<Category>(`/categories/${id}`, { method: 'PATCH', body: JSON.stringify(body) }),
  deleteCategory: (id: string) => req(`/categories/${id}`, { method: 'DELETE' }),
  bookmarks: (category_id?: string | null, include_vault = false) => {
    const q = new URLSearchParams()
    if (category_id) q.set('category_id', category_id)
    if (include_vault) q.set('include_vault', 'true')
    return req<Bookmark[]>(`/bookmarks?${q}`)
  },
  createBookmark: (body: {
    url: string
    title?: string
    description?: string
    notes?: string
    category_id?: string | null
    category_ids?: string[]
    tags: string[]
  }) => req<Bookmark>('/bookmarks', { method: 'POST', body: JSON.stringify(body) }),
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
  ) => req<Bookmark>(`/bookmarks/${id}`, { method: 'PATCH', body: JSON.stringify(body) }),
  deleteBookmark: (id: string) => req(`/bookmarks/${id}`, { method: 'DELETE' }),
  search: (q: string) => req<SearchResponse>(`/search?q=${encodeURIComponent(q)}`),
  import: (format: string, content: string) =>
    req<{ total: number; imported: number; failed: number }>('/import', {
      method: 'POST',
      body: JSON.stringify({ format, content }),
    }),
  exportUrl: (format: string) => `/api/export?format=${format}`,
  vaultStatus: () => req<{ configured: boolean; unlocked: boolean }>('/vault/status'),
  vaultSetup: (passphrase: string) =>
    req('/vault/setup', { method: 'POST', body: JSON.stringify({ passphrase }) }),
  vaultUnlock: (passphrase: string) =>
    req('/vault/unlock', { method: 'POST', body: JSON.stringify({ passphrase }) }),
  vaultLock: () => req('/vault/lock', { method: 'POST' }),
  duplicates: () => req<{ urls: string[] }>('/bookmarks/duplicates'),
}
