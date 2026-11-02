type PreviewSource = {
  id: string
  has_preview?: boolean
  /** Where to load the stored preview instead of the signed-in route, e.g. on a public list. */
  preview_url?: string | null
  preview_image_url?: string | null
  updated_at?: string | null
}

/** Local screenshot / custom upload when present; else remote OG image. */
export function bookmarkPreviewSrc(b: PreviewSource): string | undefined {
  if (b.has_preview) {
    if (b.preview_url) return b.preview_url
    const bust = b.updated_at ? `?v=${encodeURIComponent(b.updated_at)}` : ''
    return `/api/bookmarks/${b.id}/preview${bust}`
  }
  return b.preview_image_url || undefined
}

export function previewSrcCandidates(b: PreviewSource): string[] {
  const list: string[] = []
  const primary = bookmarkPreviewSrc(b)
  if (primary) list.push(primary)
  if (b.has_preview && b.preview_image_url && b.preview_image_url !== primary) {
    list.push(b.preview_image_url)
  }
  return list
}
