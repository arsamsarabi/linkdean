import { describe, expect, it } from 'vitest'
import { bookmarkSchema } from '@/schemas'
import { archiveFileUrl, archivePending, formatBytes, hasArchive, primaryArchiveFormat } from './archive'

describe('archive helpers', () => {
  it('builds the copy and download addresses', () => {
    expect(archiveFileUrl('b1', 'html')).toBe('/api/bookmarks/b1/archive/html')
    expect(archiveFileUrl('b1', 'pdf', true)).toBe('/api/bookmarks/b1/archive/pdf?download=1')
  })

  it('opens the page first, then the original file, then the PDF', () => {
    expect(primaryArchiveFormat(['pdf', 'html'])).toBe('html')
    expect(primaryArchiveFormat(['pdf', 'file'])).toBe('file')
    expect(primaryArchiveFormat(['pdf'])).toBe('pdf')
    expect(primaryArchiveFormat([])).toBeNull()
  })

  it('polls only while a copy is queued or being captured', () => {
    const row = { format: 'html', bytes: 0, available: false } as const
    expect(archivePending([{ ...row, status: 'running' }])).toBe(true)
    expect(archivePending([{ ...row, status: 'queued' }])).toBe(true)
    expect(
      archivePending([
        { ...row, status: 'failed' },
        { ...row, status: 'done' },
      ])
    ).toBe(false)
    expect(archivePending(undefined)).toBe(false)
  })

  it('counts a bookmark as archived only when a copy is stored', () => {
    expect(hasArchive(undefined)).toBe(false)
    expect(hasArchive({ formats: [], pending: true, failed: false })).toBe(false)
    expect(hasArchive({ formats: ['html'], pending: false, failed: false })).toBe(true)
  })

  it('formats sizes', () => {
    expect(formatBytes(512)).toBe('512 B')
    expect(formatBytes(2048)).toBe('2 KB')
    expect(formatBytes(3.5 * 1024 * 1024)).toBe('3.5 MB')
  })

  it('reads a bookmark with and without an archive summary', () => {
    const field = bookmarkSchema.shape.archive
    expect(field.parse(undefined)).toBeUndefined()
    const summary = { formats: ['html', 'pdf'], captured_at: '2026-11-17T00:00:00Z', pending: false, failed: false }
    expect(field.parse(summary)?.formats).toEqual(['html', 'pdf'])
    expect(field.safeParse({ ...summary, formats: ['zip'] }).success).toBe(false)
  })
})
