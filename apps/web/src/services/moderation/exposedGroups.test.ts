import { describe, expect, it } from 'vitest'
import type { ExposedContent } from '@/schemas'
import { exposedGroups, moderateItems } from './exposedGroups'

const empty: ExposedContent = {
  public_links: [],
  categories: [],
  bookmarks: [],
  profile_visibility: null,
  shares: [],
  comments: [],
}

describe('exposedGroups', () => {
  it('leaves out empty groups', () => {
    expect(exposedGroups(empty, 'u1')).toEqual([])
  })

  it('uses the user id for the profile row', () => {
    const [group] = exposedGroups({ ...empty, profile_visibility: 'members' }, 'u1')
    expect(group.rows[0].id).toBe('u1')
  })

  it('falls back to the url for an untitled bookmark', () => {
    const bookmarks = [{ id: 'b', title: '', url: 'https://a.example/' }]
    expect(exposedGroups({ ...empty, bookmarks }, 'u1')[0].rows[0].label).toBe('https://a.example/')
  })
})

describe('moderateItems', () => {
  it('builds items for selected rows, with the action subject type', () => {
    const bookmarks = [
      { id: 'a', title: 'A', url: 'https://a.example/' },
      { id: 'b', title: 'B', url: 'https://b.example/' },
    ]
    const [group] = exposedGroups({ ...empty, bookmarks }, 'u1')
    expect(moderateItems(group, group.actions[1], new Set(['b']))).toEqual([
      { action: 'removed_to_trash', subject_type: 'bookmark', subject_id: 'b' },
    ])
  })

  it('takes the subject type from a share row', () => {
    const shares = [{ kind: 'dashboard_share', id: 's', title: 'Home', invitee: 'a@b.c', access: 'read' }]
    const [group] = exposedGroups({ ...empty, shares }, 'u1')
    expect(moderateItems(group, group.actions[0], new Set(['s']))[0].subject_type).toBe('dashboard_share')
  })
})
