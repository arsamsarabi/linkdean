import type { Bookmark, Category, PageDetails } from '@/schemas'

export type BookmarkDraft = {
  id?: string
  url: string
  title: string
  description: string
  tags: string[]
  category_ids: string[]
  is_favorite: boolean
  is_unread: boolean
  is_pinned: boolean
  is_private: boolean
  has_preview?: boolean
  preview_image_url?: string | null
  updated_at?: string
  can_write?: boolean
  is_vault?: boolean
  vault_id?: string | null
  owner_user_id?: string
}

export function emptyDraft(): BookmarkDraft {
  return {
    url: '',
    title: '',
    description: '',
    tags: [],
    category_ids: [],
    is_favorite: false,
    is_unread: false,
    is_pinned: false,
    is_private: false,
  }
}

export function draftFromBookmark(b: Bookmark): BookmarkDraft {
  return {
    id: b.id,
    url: b.url,
    title: b.title || '',
    description: b.description || '',
    tags: b.tags || [],
    category_ids: (b.categories || []).map((c) => c.id),
    is_favorite: !!b.is_favorite,
    is_unread: !!b.is_unread,
    is_pinned: !!b.is_pinned,
    is_private: !!b.is_private,
    has_preview: !!b.has_preview,
    preview_image_url: b.preview_image_url,
    updated_at: b.updated_at,
    can_write: !!b.can_write,
    is_vault: !!b.is_vault,
    owner_user_id: b.owner_user_id,
  }
}

/**
 * Fills the draft's empty title, description and tags from the page's details. Anything
 * the person has already typed stays as it is.
 */
export function fillFromPage(draft: BookmarkDraft, details: PageDetails): BookmarkDraft {
  return {
    ...draft,
    title: draft.title.trim() ? draft.title : (details.title ?? draft.title),
    description: draft.description.trim() ? draft.description : (details.description ?? draft.description),
    tags: draft.tags.length > 0 ? draft.tags : [...details.tags],
  }
}

/** True when saving this draft puts the bookmark in a vault, whose URLs never leave the instance. */
export function draftHeadsToVault(draft: BookmarkDraft, categories: Category[], vaultRequired = false): boolean {
  if (vaultRequired || draft.is_vault || draft.vault_id) return true
  return draft.category_ids.some((id) => categories.find((c) => c.id === id)?.is_vault)
}

/** True when `url` is an http(s) address with a host, so the server can be asked to fetch it. */
export function isFetchableUrl(url: string): boolean {
  try {
    const u = new URL(url)
    return (u.protocol === 'http:' || u.protocol === 'https:') && u.hostname.length > 0
  } catch {
    return false
  }
}
