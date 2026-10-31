/** Local screenshot when present; else remote OG image. */
export function bookmarkPreviewSrc(b: {
  id: string
  has_preview?: boolean
  preview_image_url?: string | null
}): string | undefined {
  if (b.has_preview) return `/api/bookmarks/${b.id}/preview`
  return b.preview_image_url || undefined
}

export function previewSrcCandidates(b: {
  id: string
  has_preview?: boolean
  preview_image_url?: string | null
}): string[] {
  const list: string[] = []
  const primary = bookmarkPreviewSrc(b)
  if (primary) list.push(primary)
  if (b.has_preview && b.preview_image_url && b.preview_image_url !== primary) {
    list.push(b.preview_image_url)
  }
  return list
}
