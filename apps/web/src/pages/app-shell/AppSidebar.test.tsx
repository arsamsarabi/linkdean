import type { ReactNode } from 'react'
import { AppShell, MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { useShellUi } from '@/stores'
import { AppSidebar } from './AppSidebar'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, search, children, ...rest }: { to: string; search?: Record<string, string>; children: ReactNode }) => (
    <a href={search ? `${to}?${new URLSearchParams(search)}` : to} {...rest}>
      {children}
    </a>
  ),
}))

const folders = vi.hoisted(() => [
  { id: 'f1', name: 'Design', parent_id: null, is_hidden: false, is_vault: false, access: 'owner', shared_out: false },
  { id: 'f2', name: 'Fonts', parent_id: 'f1', is_hidden: false, is_vault: false, access: 'owner', shared_out: false },
  { id: 'f3', name: 'From Ana', parent_id: null, is_hidden: false, is_vault: false, access: 'read', shared_out: false },
  { id: 'v1', name: 'Secret', parent_id: null, is_hidden: false, is_vault: true, access: 'owner', shared_out: false },
])

vi.mock('@/hooks', () => ({
  usePrefetchBrokenLinks: () => () => undefined,
  usePrefetchActivity: () => () => undefined,
  useCategories: () => ({ isLoading: false, data: folders }),
  useShareInvites: () => ({ data: { invites: [{ id: 'i1', rejected_at: null }] } }),
}))

beforeAll(() => {
  // The sidebar's ScrollArea measures itself; jsdom has no ResizeObserver.
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})

function renderSidebar(
  opts: { vaultUnlocked?: boolean; isAdmin?: boolean; pathname?: string; search?: Record<string, unknown> } = {}
) {
  return render(
    <MantineProvider env="test">
      <AppShell navbar={{ width: 248, breakpoint: 'sm' }}>
        <AppShell.Navbar>
          <AppSidebar
            pathname={opts.pathname ?? '/'}
            search={opts.search ?? {}}
            vaultUnlocked={opts.vaultUnlocked ?? false}
            isAdmin={opts.isAdmin ?? false}
          />
        </AppShell.Navbar>
      </AppShell>
    </MantineProvider>
  )
}

describe('AppSidebar', () => {
  beforeEach(() => useShellUi.setState({ navOpen: true }))
  afterEach(() => {
    cleanup()
    localStorage.clear()
  })

  it('links to Vaults while a vault is open', () => {
    renderSidebar({ vaultUnlocked: true })
    expect(screen.getByRole('link', { name: 'Vaults' }).getAttribute('href')).toBe('/vaults')
  })

  it('hides the Vaults link when every vault is locked', () => {
    renderSidebar()
    expect(screen.queryByRole('link', { name: 'Vaults' })).toBeNull()
  })

  it('links each quick view to the filtered library and marks the open one', () => {
    renderSidebar({ pathname: '/library', search: { flag: 'favorite' } })
    expect(screen.getByRole('link', { name: 'Reading list' }).getAttribute('href')).toBe('/library?flag=unread')
    expect(screen.getByRole('link', { name: 'Favorites' }).hasAttribute('data-active')).toBe(true)
    expect(screen.getByRole('link', { name: 'Library' }).hasAttribute('data-active')).toBe(false)
  })

  it('links the library health pages, marks the open one, and closes the phone menu', async () => {
    renderSidebar({ pathname: '/duplicates' })
    expect(screen.getByRole('link', { name: 'Broken links' }).getAttribute('href')).toBe('/broken-links')
    expect(screen.getByRole('link', { name: 'History' }).getAttribute('href')).toBe('/history')
    expect(screen.getByRole('link', { name: 'Duplicates' }).hasAttribute('data-active')).toBe(true)
    await userEvent.click(screen.getByRole('link', { name: 'Broken links' }))
    expect(useShellUi.getState().navOpen).toBe(false)
  })

  it('lists folders with Unsorted first, nests children, and leaves vault folders out', async () => {
    renderSidebar({ pathname: '/library', search: { folder: 'f2' } })
    await userEvent.click(screen.getByRole('button', { name: 'Folders' }))
    expect(screen.getByRole('link', { name: 'Unsorted' }).getAttribute('href')).toBe(
      '/library?folder=uncategorized&category=uncategorized'
    )
    expect(screen.getByRole('link', { name: 'Design' }).getAttribute('href')).toBe('/library?folder=f1')
    // The open folder's parent starts expanded, so the open folder shows and is marked.
    expect(screen.getByRole('link', { name: 'Fonts' }).hasAttribute('data-active')).toBe(true)
    expect(screen.getByRole('link', { name: /From Ana/ })).toBeTruthy()
    expect(screen.getByLabelText('Shared')).toBeTruthy()
    expect(screen.queryByRole('link', { name: 'Secret' })).toBeNull()
    expect(screen.getByRole('link', { name: 'Library' }).hasAttribute('data-active')).toBe(false)
  })

  it('keeps Deleted items and settings reachable', () => {
    renderSidebar()
    expect(screen.getByRole('link', { name: 'Deleted items' }).getAttribute('href')).toBe('/trash')
    expect(screen.getByRole('link', { name: 'Config options' }).getAttribute('href')).toBe('/settings')
  })

  it('links Admin for admins only', () => {
    renderSidebar()
    expect(screen.queryByRole('link', { name: 'Admin' })).toBeNull()
    cleanup()
    renderSidebar({ isAdmin: true })
    expect(screen.getByRole('link', { name: 'Admin' }).getAttribute('href')).toBe('/admin')
  })

  it('links Shared, with the number of invites waiting', () => {
    renderSidebar()
    const hrefs = screen.getAllByRole('link', { name: /Shared/ }).map((a) => a.getAttribute('href'))
    expect(hrefs).toContain('/shared')
    expect(screen.getByLabelText('1 invites waiting')).toBeTruthy()
  })

  it('starts collapsed, showing recent folders and the open one', () => {
    renderSidebar({ pathname: '/library', search: { folder: 'f2' } })
    expect(screen.getByRole('button', { name: 'Folders' }).getAttribute('aria-expanded')).toBe('false')
    expect(screen.getByRole('link', { name: 'Unsorted' })).toBeTruthy()
    expect(screen.getByRole('link', { name: 'Fonts' }).hasAttribute('data-active')).toBe(true)
    expect(screen.queryByRole('searchbox', { name: 'Filter folders' })).toBeNull()
  })

  it('filters folders by name when expanded', async () => {
    renderSidebar()
    await userEvent.click(screen.getByRole('button', { name: 'Folders' }))
    expect(screen.getByRole('searchbox', { name: 'Filter folders' }).getAttribute('autocomplete')).toBe('off')
    await userEvent.type(screen.getByRole('searchbox', { name: 'Filter folders' }), 'fon')
    expect(screen.getByRole('link', { name: /Fonts/ }).getAttribute('href')).toBe('/library?folder=f2')
    expect(screen.getByText('Design / Fonts')).toBeTruthy()
    expect(screen.queryByRole('link', { name: 'Unsorted' })).toBeNull()
    await userEvent.type(screen.getByRole('searchbox', { name: 'Filter folders' }), 'zzz')
    expect(screen.getByText('No folders match.')).toBeTruthy()
  })
})
