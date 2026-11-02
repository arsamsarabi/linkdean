import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import { emptyDraft } from '@/services'
import { BookmarkDrawer } from './BookmarkDrawer'

const idle = { isPending: false, mutateAsync: vi.fn() }

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useCreateBookmark: () => idle,
  useUpdateBookmark: () => idle,
  useUploadBookmarkPreview: () => idle,
  useDeleteBookmarkPreview: () => idle,
  useMe: () => ({ data: { user: { id: 'me' } } }),
  useVaults: () => ({ data: { vaults: [] } }),
}))

vi.mock('@/components/social', () => ({
  SocialThread: ({ subjectId, enabled }: { subjectId: string; enabled: boolean }) => (
    <div>{enabled ? `thread for ${subjectId}` : 'thread idle'}</div>
  ),
}))

vi.mock('./BookmarkPreviewField', () => ({ BookmarkPreviewField: () => null }))

beforeAll(() => {
  // The description field autosizes, and listens for web fonts; jsdom has neither.
  Object.defineProperty(document, 'fonts', {
    configurable: true,
    value: { addEventListener: () => undefined, removeEventListener: () => undefined },
  })
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})

function renderDrawer(opts: { id?: string; discussion?: boolean }): void {
  const initial = opts.id ? { ...emptyDraft(), id: opts.id, url: 'https://react.dev', owner_user_id: 'sam' } : null
  render(
    <MantineProvider env="test">
      <BookmarkDrawer opened onClose={() => undefined} categories={[]} initial={initial} discussion={opts.discussion} />
    </MantineProvider>
  )
}

describe('BookmarkDrawer discussion', () => {
  afterEach(cleanup)

  it('shows the discussion of a shared bookmark in its own tab, loaded only when opened', async () => {
    renderDrawer({ id: 'b1', discussion: true })
    expect(screen.getByLabelText(/^URL/)).toBeTruthy()
    await userEvent.click(screen.getByRole('tab', { name: 'Discussion' }))
    expect(await screen.findByText('thread for b1')).toBeTruthy()
  })

  it('has no tabs for a bookmark that is not shared, or a new one', () => {
    renderDrawer({ id: 'b1' })
    expect(screen.queryByRole('tab', { name: 'Discussion' })).toBeNull()
    cleanup()
    renderDrawer({ discussion: true })
    expect(screen.queryByRole('tab', { name: 'Discussion' })).toBeNull()
  })
})
