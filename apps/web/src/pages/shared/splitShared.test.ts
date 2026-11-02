import { describe, expect, it } from 'vitest'
import type { Category, DashboardSummary } from '@/schemas'
import { splitShared } from './splitShared'

function cat(id: string, over: Partial<Category> = {}): Category {
  return {
    id,
    name: id,
    is_hidden: false,
    visibility: 'private',
    is_vault: false,
    position: 0,
    access: 'owner',
    shared_out: false,
    shared_with: [],
    ...over,
  } as Category
}

function board(id: string, over: Partial<DashboardSummary> = {}): DashboardSummary {
  return {
    id,
    owner_user_id: 'u',
    title: id,
    is_default: false,
    access: 'owner',
    shared_out: false,
    shared_with: [],
    ...over,
  } as DashboardSummary
}

describe('splitShared', () => {
  it('lists incoming folders once, at their top, and outgoing ones with their people', () => {
    const { withMe, byMe } = splitShared(
      [
        cat('Recipes', { access: 'read', owner_display_name: 'Sam' }),
        cat('Desserts', { access: 'read', parent_id: 'Recipes' }),
        cat('Design', { shared_out: true, shared_with: ['Alex', 'Kim'] }),
        cat('Typography', { shared_out: true, shared_with: ['Alex', 'Kim'], parent_id: 'Design' }),
        cat('Mine'),
        cat('Gone', { access: 'write', deleted_at: '2026-01-01' }),
        cat('Locked', { access: 'read', is_vault: true }),
      ],
      [board('Team', { access: 'write', owner_display_name: 'Lee' }), board('Home', { shared_out: true })]
    )
    expect(withMe.map((i) => [i.title, i.people, i.access])).toEqual([
      ['Recipes', 'from Sam', 'read'],
      ['Team', 'from Lee', 'write'],
    ])
    expect(byMe.map((i) => [i.kind, i.title, i.people])).toEqual([
      ['folder', 'Design', 'with Alex, Kim'],
      ['dashboard', 'Home', 'shared'],
    ])
  })
})
