import { describe, expect, it } from 'vitest'
import { buildPaletteItems, clampActiveIndex } from './commandPalette'

const cats = [
  { id: '1', label: 'Work / Docs' },
  { id: '2', label: 'Personal', is_hidden: true },
  { id: '3', label: 'Recipes' },
]

describe('buildPaletteItems', () => {
  it('shows add + search + folders when empty', () => {
    const items = buildPaletteItems('', cats)
    expect(items.map((i) => i.id)).toEqual([
      'action:add',
      'action:search',
      'jump:all',
      'jump:uncategorized',
      'cat:1',
      'cat:3',
    ])
  })

  it('includes hidden when showHidden', () => {
    const items = buildPaletteItems('personal', cats, { showHidden: true })
    expect(items.some((i) => i.id === 'cat:2')).toBe(true)
  })

  it('filters categories by query', () => {
    const items = buildPaletteItems('recipe', cats)
    expect(items.filter((i) => i.kind === 'jump-category').map((i) => i.id)).toEqual(['cat:3'])
  })

  it('search label includes query', () => {
    const search = buildPaletteItems('rust', cats).find((i) => i.kind === 'search')
    expect(search?.label).toContain('rust')
  })
})

describe('clampActiveIndex', () => {
  it('wraps', () => {
    expect(clampActiveIndex(-1, 3)).toBe(2)
    expect(clampActiveIndex(3, 3)).toBe(0)
    expect(clampActiveIndex(1, 0)).toBe(0)
  })
})
