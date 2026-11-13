import { MantineProvider } from '@mantine/core'
import { cleanup, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { BookmarkCardPreview } from './BookmarkCardPreview'
import { GlanceTile } from './GlanceTile'

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useRecordBookmarkOpen: () => vi.fn(),
}))

function images(ui: React.ReactElement): string[] {
  const { container } = render(<MantineProvider env="test">{ui}</MantineProvider>)
  return Array.from(container.querySelectorAll('img'), (i) => i.getAttribute('src') ?? '')
}

const url = 'https://secret.example/path'
const noop = (): void => {}

describe('third-party favicon lookups', () => {
  afterEach(cleanup)

  it('never send a vault hostname to Google', () => {
    const card = <BookmarkCardPreview url={url} title="t" previewSrc={undefined} isVault onPreviewError={noop} />
    expect(images(card).join()).not.toContain('google')
  })

  it('still run for ordinary bookmarks', () => {
    const card = <BookmarkCardPreview url={url} title="t" previewSrc={undefined} onPreviewError={noop} />
    expect(images(card).join()).toContain('google.com/s2/favicons')
  })

  it('never run for a vault tile', () => {
    const tile = <GlanceTile bookmark={{ id: '1', url, title: 't', is_vault: true }} />
    expect(images(tile).join()).not.toContain('google')
  })
})
