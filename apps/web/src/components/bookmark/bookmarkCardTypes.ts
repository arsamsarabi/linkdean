import type { CategoryRef } from '@/schemas'

export type BookmarkCardItem = {
  id: string
  url: string
  title: string
  description?: string | null
  favicon_url?: string | null
  preview_image_url?: string | null
  has_preview?: boolean
  /** Public lists: where anyone may load the stored preview. */
  preview_url?: string | null
  updated_at?: string
  is_vault: boolean
  tags: string[]
  categories?: CategoryRef[]
  owner_user_id?: string
  /** When false, edit/delete stay hidden (read-only share). Default true for legacy callers. */
  can_write?: boolean
  /** Owner has shared a folder this bookmark lives in. */
  shared_out?: boolean
  /** Owner-only: invitee labels for outgoing share tooltip. */
  shared_with?: string[]
  /** Invitee-only: owner's display name for share badge. */
  owner_display_name?: string | null
  is_favorite?: boolean
  is_unread?: boolean
  is_pinned?: boolean
  deleted_at?: string | null
  link_check_status?: 'ok' | 'broken' | 'unreachable' | 'unverifiable' | null
  link_check_status_code?: number | null
  link_check_error?: string | null
}
