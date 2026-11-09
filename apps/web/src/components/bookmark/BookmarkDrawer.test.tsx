import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import { emptyDraft } from '@/services'
import { BookmarkDrawer } from './BookmarkDrawer'

const idle = { isPending: false, mutateAsync: vi.fn() }
const create = { isPending: false, mutateAsync: vi.fn() }
const pageDetails = { isPending: false, mutateAsync: vi.fn() }

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useCreateBookmark: () => create,
  useUpdateBookmark: () => idle,
  useUploadBookmarkPreview: () => idle,
  useDeleteBookmarkPreview: () => idle,
  useMe: () => ({ data: { user: { id: 'me' } } }),
  useVaults: () => ({ data: { vaults: [] } }),
  usePageDetails: () => pageDetails,
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

function renderDrawer(opts: { id?: string; discussion?: boolean; vaultRequired?: boolean }): void {
  const initial = opts.id ? { ...emptyDraft(), id: opts.id, url: 'https://react.dev', owner_user_id: 'sam' } : null
  render(
    <MantineProvider env="test">
      <BookmarkDrawer
        opened
        onClose={() => undefined}
        categories={[]}
        initial={initial}
        discussion={opts.discussion}
        vaultRequired={opts.vaultRequired}
      />
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

describe('BookmarkDrawer Import from URL', () => {
  afterEach(() => {
    cleanup()
    pageDetails.mutateAsync.mockReset()
  })

  it('stays disabled until the URL is a web address, then fills only the empty fields', async () => {
    pageDetails.mutateAsync.mockResolvedValue({ title: 'From page', description: 'About it', tags: ['x'] })
    renderDrawer({})
    const button = screen.getByRole('button', { name: /Import from URL/ })
    expect(button).toHaveProperty('disabled', true)

    await userEvent.type(screen.getByLabelText(/^URL/), 'example.com/post')
    await userEvent.type(screen.getByLabelText('Title'), 'My title')
    expect(button).toHaveProperty('disabled', false)
    await userEvent.click(button)

    expect(pageDetails.mutateAsync).toHaveBeenCalledWith('https://example.com/post')
    expect((screen.getByLabelText('Title') as HTMLInputElement).value).toBe('My title')
    expect((screen.getByLabelText('Description') as HTMLTextAreaElement).value).toBe('About it')
  })

  it('shows why a page could not be fetched, and keeps the form usable', async () => {
    pageDetails.mutateAsync.mockRejectedValue(new Error("Couldn't reach that page."))
    renderDrawer({})
    await userEvent.type(screen.getByLabelText(/^URL/), 'https://example.com')
    await userEvent.click(screen.getByRole('button', { name: /Import from URL/ }))
    expect((await screen.findByRole('alert')).textContent).toContain("Couldn't reach that page.")
    expect(screen.getByRole('button', { name: 'Add bookmark' })).toHaveProperty('disabled', false)
  })

  it('is not offered for a bookmark going into a vault', () => {
    renderDrawer({ vaultRequired: true })
    expect(screen.queryByRole('button', { name: /Import from URL/ })).toBeNull()
  })
})

describe('BookmarkDrawer quick save', () => {
  afterEach(() => {
    cleanup()
    pageDetails.mutateAsync.mockReset()
    create.mutateAsync.mockReset()
  })

  function renderQuick(initial: ReturnType<typeof emptyDraft>, onSaved = vi.fn(), onClose = vi.fn()): void {
    render(
      <MantineProvider env="test">
        <BookmarkDrawer
          opened
          onClose={onClose}
          categories={[]}
          initial={initial}
          autoImport
          quickSave
          onSaved={onSaved}
        />
      </MantineProvider>
    )
  }

  it('fetches the page once when it opens without a title', async () => {
    pageDetails.mutateAsync.mockResolvedValue({ title: 'Fetched', description: null, tags: [] })
    renderQuick({ ...emptyDraft(), url: 'https://example.com/a' })
    expect(await screen.findByDisplayValue('Fetched')).toBeTruthy()
    expect(pageDetails.mutateAsync).toHaveBeenCalledTimes(1)
    expect(pageDetails.mutateAsync).toHaveBeenCalledWith('https://example.com/a')
  })

  it('does not fetch when a title came with the link', () => {
    renderQuick({ ...emptyDraft(), url: 'https://example.com/a', title: 'Given' })
    expect(pageDetails.mutateAsync).not.toHaveBeenCalled()
  })

  it('Save to Unsorted saves with no folder and says so to the caller', async () => {
    pageDetails.mutateAsync.mockResolvedValue({ title: null, description: null, tags: [] })
    create.mutateAsync.mockResolvedValue({ id: 'n1', is_vault: false })
    const onSaved = vi.fn()
    const onClose = vi.fn()
    renderQuick({ ...emptyDraft(), url: 'https://example.com/a', title: 'T', category_ids: ['c1'] }, onSaved, onClose)
    await userEvent.click(screen.getByRole('button', { name: 'Save to Unsorted' }))
    expect(create.mutateAsync).toHaveBeenCalledWith(expect.objectContaining({ category_ids: [], vault_id: null }))
    expect(onSaved).toHaveBeenCalledWith(expect.objectContaining({ category_ids: [] }), true)
    expect(onClose).toHaveBeenCalled()
  })
})
