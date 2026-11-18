import type { ArchiveFormat, ArchiveSummary, BookmarkArchive } from '@/schemas'

/** A stored copy. The session cookie authorises it, so it works as a frame or link `src`. */
export function archiveFileUrl(bookmarkId: string, format: ArchiveFormat, download = false): string {
  return `/api/bookmarks/${bookmarkId}/archive/${format}${download ? '?download=1' : ''}`
}

/** Reader-mode Markdown derived from the HTML archive (always a download). */
export function archiveMarkdownUrl(bookmarkId: string): string {
  return `/api/bookmarks/${bookmarkId}/archive/html?download=1&as=md`
}

/** The copy the viewer opens first: the page as HTML, then the original file, then the PDF. */
export function primaryArchiveFormat(formats: ArchiveFormat[]): ArchiveFormat | null {
  return (['html', 'file', 'pdf'] as const).find((f) => formats.includes(f)) ?? null
}

export function hasArchive(summary: ArchiveSummary | null | undefined): boolean {
  return !!summary && summary.formats.length > 0
}

/** Any copy queued or being captured, so its status is worth polling. */
export function archivePending(archives: BookmarkArchive[] | undefined): boolean {
  return !!archives?.some((a) => a.status === 'queued' || a.status === 'running')
}

export const ARCHIVE_FORMAT_LABELS: Record<ArchiveFormat, string> = {
  html: 'Web page (HTML)',
  pdf: 'PDF',
  file: 'Original file',
}

/** 3 Nov 2026 */
export function formatArchiveDate(iso: string | null | undefined): string {
  if (!iso) return ''
  return new Date(iso).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' })
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}
