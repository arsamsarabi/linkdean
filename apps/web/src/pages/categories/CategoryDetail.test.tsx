import type { ReactNode } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Category } from '@/schemas'
import { CategoryDetail } from './CategoryDetail'

vi.mock('@tanstack/react-router', () => ({
  Link: ({ to, search, children, ...rest }: { to: string; search?: Record<string, string>; children: ReactNode }) => (
    <a href={search ? `${to}?${new URLSearchParams(search)}` : to} {...rest}>
      {children}
    </a>
  ),
}))

const base: Category = {
  id: 'c1',
  parent_id: null,
  name: 'Design',
  is_hidden: false,
  visibility: 'private',
  is_vault: false,
  position: 0,
  access: 'owner',
  shared_out: false,
  shared_with: [],
}

const actions = {
  onEdit: vi.fn(),
  onNewSubcategory: vi.fn(),
  onShare: vi.fn(),
  onDuplicate: vi.fn(),
  onMoveToVault: vi.fn(),
  onSocial: vi.fn(),
  onDelete: vi.fn(),
  duplicatePending: false,
}

function renderDetail(category: Category): void {
  render(
    <MantineProvider>
      <CategoryDetail
        category={category}
        ancestors={[]}
        subcategories={[]}
        childCounts={new Map()}
        onSelect={vi.fn()}
        actions={actions}
      />
    </MantineProvider>
  )
}

afterEach(cleanup)

describe('CategoryDetail', () => {
  it('gives the owner every action and links to the library folder', () => {
    renderDetail(base)
    for (const name of ['Edit', 'New subcategory', 'Share', 'Duplicate', 'Move to vault', 'Delete']) {
      expect(screen.getByRole('button', { name })).toBeTruthy()
    }
    expect(screen.getByRole('link', { name: 'Open in library' }).getAttribute('href')).toBe('/library?folder=c1')
  })

  it('leaves a read-only share with duplicate and comments only', () => {
    renderDetail({ ...base, access: 'read' })
    expect(screen.getByRole('button', { name: 'Duplicate' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Likes & comments' })).toBeTruthy()
    for (const name of ['Edit', 'New subcategory', 'Share', 'Move to vault', 'Delete']) {
      expect(screen.queryByRole('button', { name })).toBeNull()
    }
  })
})
