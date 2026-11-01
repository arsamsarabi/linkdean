import { describe, expect, it } from 'vitest'
import { bookmarkPreviewSrc, previewSrcCandidates } from './bookmarkPreview'

describe('bookmarkPreviewSrc', () => {
  it('prefers local preview', () => {
    expect(
      bookmarkPreviewSrc({
        id: '1',
        has_preview: true,
        preview_image_url: 'https://cdn/og.png',
      })
    ).toBe('/api/bookmarks/1/preview')
  })

  it('cache-busts with updated_at', () => {
    expect(
      bookmarkPreviewSrc({
        id: '1',
        has_preview: true,
        updated_at: '2024-01-02T00:00:00Z',
      })
    ).toBe('/api/bookmarks/1/preview?v=2024-01-02T00%3A00%3A00Z')
  })

  it('falls back to remote OG', () => {
    expect(
      bookmarkPreviewSrc({
        id: '1',
        has_preview: false,
        preview_image_url: 'https://cdn/og.png',
      })
    ).toBe('https://cdn/og.png')
  })
})

describe('previewSrcCandidates', () => {
  it('includes remote fallback after local', () => {
    expect(
      previewSrcCandidates({
        id: '1',
        has_preview: true,
        preview_image_url: 'https://cdn/og.png',
      })
    ).toEqual(['/api/bookmarks/1/preview', 'https://cdn/og.png'])
  })
})
