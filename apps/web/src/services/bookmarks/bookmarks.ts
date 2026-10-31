import type { Bookmark } from '@/schemas'

export type BookmarkDraft = {
  id?: string
  url: string
  title: string
  description: string
  tags: string[]
  category_ids: string[]
}

export function emptyDraft(): BookmarkDraft {
  return {
    url: '',
    title: '',
    description: '',
    tags: [],
    category_ids: [],
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
  }
}
