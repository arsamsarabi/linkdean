import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { Dashboard } from '@/schemas'
import { useDashboardEdit } from './useDashboardEdit'

vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))

const item = (id: string) => ({ bookmark: { id, title: id, url: `https://${id}.example`, is_vault: false } })
const section = (id: string, ids: string[]) => ({
  id,
  title: id,
  position: 0,
  kind: 'pinned',
  display: 'medium',
  item_limit: 8,
  source: { category_id: null, tag: null, site: null, favorites: false, unread: false },
  open_window: 'all_time',
  width: 'full',
  items: ids.map(item),
})
const dashboard = {
  id: 'd1',
  title: 'Home',
  sections: [section('one', ['a', 'b', 'c']), section('two', []), section('three', [])],
} as unknown as Dashboard

describe('useDashboardEdit drag moves', () => {
  it('moves a section to where it was dropped and reorders pins within a section', () => {
    const save = { mutateAsync: vi.fn() }
    const { result } = renderHook(() => useDashboardEdit({ activeId: 'd1', dashboard, save }))
    act(() => result.current.startEdit())
    act(() => result.current.moveSectionTo(0, 2))
    expect(result.current.editDraft!.sections.map((s) => s.title)).toEqual(['two', 'three', 'one'])
    act(() => result.current.movePin(2, 2, 0))
    expect(result.current.editDraft!.sections[2]!.bookmark_ids).toEqual(['c', 'a', 'b'])
  })

  it('returns the key of an added section so its settings can open', () => {
    const save = { mutateAsync: vi.fn() }
    const { result } = renderHook(() => useDashboardEdit({ activeId: 'd1', dashboard, save }))
    act(() => result.current.startEdit())
    let key = ''
    act(() => {
      key = result.current.addSection('latest')
    })
    expect(result.current.editDraft!.sections.at(-1)!.key).toBe(key)
  })
})
