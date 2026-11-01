import type { Bookmark } from '@/schemas'

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
  has_preview?: boolean
  preview_image_url?: string | null
  updated_at?: string
  can_write?: boolean
  is_vault?: boolean
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
    has_preview: !!b.has_preview,
    preview_image_url: b.preview_image_url,
    updated_at: b.updated_at,
    can_write: !!b.can_write,
    is_vault: !!b.is_vault,
  }
}
