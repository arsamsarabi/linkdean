import type { ReactNode } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { SharedPage } from './SharedPage'

const data = vi.hoisted(() => ({
  invites: [] as { id: string; rejected_at: null }[],
}))

vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, search, children, ...rest }: { to: string; search?: Record<string, string>; children: ReactNode }) => (
    <a
      href={search ? `${to}?${new URLSearchParams(Object.entries(search).filter(([, v]) => v !== undefined))}` : to}
      {...rest}
    >
      {children}
    </a>
  ),
}))

vi.mock('@/components', () => ({
  QueryErrorAlert: () => null,
  ShareInvitesList: () => <div>invite list</div>,
}))

vi.mock('@/hooks', () => ({
  useCategories: () => ({
    isLoading: false,
    data: [
      { id: 'c1', name: 'Recipes', access: 'read', owner_display_name: 'Sam', shared_with: [] },
      { id: 'c2', name: 'Design', access: 'owner', shared_out: true, shared_with: ['Alex'] },
    ],
  }),
  useDashboards: () => ({ isLoading: false, data: [] }),
  useShareInvites: () => ({ data: { invites: data.invites } }),
}))

function renderPage(): void {
  render(
    <MantineProvider env="test">
      <SharedPage />
    </MantineProvider>
  )
}

describe('SharedPage', () => {
  afterEach(cleanup)

  it('links what is shared with me and by me to its folder', () => {
    renderPage()
    expect(screen.getByRole('link', { name: 'Recipes' }).getAttribute('href')).toBe('/library?folder=c1')
    expect(screen.getByText('Read only')).toBeTruthy()
    expect(screen.getByText('Folder · with Alex')).toBeTruthy()
    expect(screen.getByText('No invites waiting.')).toBeTruthy()
  })

  it('shows invites when some wait', () => {
    data.invites = [{ id: 'i1', rejected_at: null }]
    renderPage()
    expect(screen.getByText('invite list')).toBeTruthy()
    data.invites = []
  })
})
