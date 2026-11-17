import { useCallback } from 'react'
import { useRecordBookmarkOpen } from '@/hooks'
import { hasArchive } from '@/lib'
import type { ArchiveSummary } from '@/schemas'
import { bookmarkPreviewSrc } from '@/services'
import { canPeekBookmark, usePeekUi } from '@/stores'

export type PeekableBookmark = {
  id: string
  url: string
  title: string
  description?: string | null
  is_vault?: boolean
  has_preview?: boolean
  preview_url?: string | null
  preview_image_url?: string | null
  updated_at?: string | null
  archive?: ArchiveSummary | null
}

/** Opens the peek panel and records an open. No-op for vault / non-https. */
export function useOpenPeek(): (b: PeekableBookmark) => void {
  const recordOpen = useRecordBookmarkOpen()
  const open = usePeekUi((s) => s.open)

  return useCallback(
    (b: PeekableBookmark) => {
      if (!canPeekBookmark(b)) return
      recordOpen(b.id)
      open({
        bookmarkId: b.id,
        url: b.url,
        title: b.title,
        description: b.description,
        previewUrl: bookmarkPreviewSrc(b) ?? null,
        archived: hasArchive(b.archive),
      })
    },
    [open, recordOpen]
  )
}
