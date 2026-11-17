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

describe('favicons', () => {
  afterEach(cleanup)

  it('never send a hostname to a third party', () => {
    const card = <BookmarkCardPreview url={url} title="t" previewSrc={undefined} onPreviewError={noop} />
    const tile = <GlanceTile bookmark={{ id: '1', url, title: 't', is_vault: false }} />
    expect([...images(card), ...images(tile)]).toEqual([])
  })
})
