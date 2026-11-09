import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { BookmarkDraft } from '@/services'
import { SavePage } from './SavePage'

const search = vi.hoisted(() => ({ value: {} as Record<string, string | boolean | undefined> }))
const lookup = vi.hoisted(() => ({ value: { isPending: false, data: null as unknown } }))
const drawer = vi.hoisted(() => ({ props: null as null | Record<string, unknown> }))

vi.mock('@tanstack/react-router', () => ({
  useSearch: () => search.value,
  useNavigate: () => vi.fn(),
}))

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useCategories: () => ({
    isPending: false,
    data: [
      { id: 'f1', name: 'Reading', is_vault: false, access: 'owner' },
      { id: 'v1', name: 'Vault', is_vault: true, access: 'owner' },
    ],
  }),
  useBookmarkByUrl: () => lookup.value,
}))

vi.mock('@/components', () => ({
  BookmarkDrawer: (props: Record<string, unknown>) => {
    drawer.props = props
    const initial = props.initial as BookmarkDraft
    return (
      <div>
        <div>drawer {initial.url}</div>
        {props.notice as React.ReactNode}
      </div>
    )
  },
}))

function renderPage(): void {
  render(
    <MantineProvider env="test">
      <SavePage />
    </MantineProvider>
  )
}

function closeDrawer(): void {
  const onClose = drawer.props?.onClose as (() => void) | undefined
  onClose?.()
}

describe('SavePage', () => {
  beforeEach(() => {
    localStorage.clear()
    drawer.props = null
    lookup.value = { isPending: false, data: null }
  })
  afterEach(cleanup)

  it('opens a new bookmark from the link, in the last folder used', () => {
    localStorage.setItem('linkdean_last_save_folder', 'f1')
    search.value = { url: 'example.com/post', title: 'Post', text: 'Quote' }
    renderPage()
    expect(screen.getByText('drawer https://example.com/post')).toBeTruthy()
    expect(drawer.props?.initial).toMatchObject({ title: 'Post', description: 'Quote', category_ids: ['f1'] })
    expect(drawer.props).toMatchObject({ autoImport: true, quickSave: true })
  })

  it('does not start in a folder that is gone or is a vault', () => {
    localStorage.setItem('linkdean_last_save_folder', 'v1')
    search.value = { url: 'https://example.com' }
    renderPage()
    expect(drawer.props?.initial).toMatchObject({ category_ids: [] })
  })

  it('opens the bookmark already saved at that address', () => {
    lookup.value = {
      isPending: false,
      data: { id: 'b9', url: 'https://example.com/', title: 'Old', tags: [], categories: [] },
    }
    search.value = { url: 'https://example.com/' }
    renderPage()
    expect(screen.getByText('Already in your library')).toBeTruthy()
    expect(drawer.props?.initial).toMatchObject({ id: 'b9', title: 'Old' })
    expect(drawer.props).toMatchObject({ autoImport: false, quickSave: false })
  })

  it('remembers the folder of a normal save, not of Save to Unsorted', () => {
    search.value = { url: 'https://example.com' }
    renderPage()
    const onSaved = drawer.props?.onSaved as (d: Partial<BookmarkDraft>, unsorted: boolean) => void
    onSaved({ category_ids: [] }, true)
    expect(localStorage.getItem('linkdean_last_save_folder')).toBeNull()
    onSaved({ category_ids: ['f1'] }, false)
    expect(localStorage.getItem('linkdean_last_save_folder')).toBe('f1')
  })

  it('closes the bookmarklet window when done, and only that window', () => {
    const closeWindow = vi.spyOn(window, 'close').mockImplementation(() => undefined)
    search.value = { url: 'https://example.com', popup: true }
    renderPage()
    closeDrawer()
    expect(closeWindow).toHaveBeenCalledTimes(1)
    cleanup()
    search.value = { url: 'https://example.com' }
    renderPage()
    closeDrawer()
    expect(closeWindow).toHaveBeenCalledTimes(1)
    closeWindow.mockRestore()
  })

  it('explains when the link has no web address', () => {
    search.value = { text: 'just words' }
    renderPage()
    expect(screen.getByText('Nothing to save')).toBeTruthy()
    expect(drawer.props).toBeNull()
  })
})
