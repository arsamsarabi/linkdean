import { describe, expect, it } from 'vitest'
import { CATEGORY_ICON_OPTIONS, filterCategoryIcons, isCategoryIconKey } from './categoryIcons'

describe('categoryIcons', () => {
  it('keeps stable legacy keys', () => {
    for (const key of [
      'bookmark',
      'star',
      'heart',
      'code',
      'book',
      'briefcase',
      'home',
      'music',
      'photo',
      'world',
      'gamepad',
      'cart',
      'school',
      'flask',
      'plane',
      'car',
      'pizza',
      'coffee',
      'movie',
      'news',
      'tools',
      'lock',
    ]) {
      expect(isCategoryIconKey(key)).toBe(true)
    }
  })

  it('exposes a large curated catalog', () => {
    expect(CATEGORY_ICON_OPTIONS.length).toBeGreaterThan(100)
    const keys = new Set(CATEGORY_ICON_OPTIONS.map((o) => o.key))
    expect(keys.size).toBe(CATEGORY_ICON_OPTIONS.length)
  })

  it('filters by label and key', () => {
    const byLabel = filterCategoryIcons('rocket')
    expect(byLabel.some((o) => o.key === 'rocket')).toBe(true)

    const byKey = filterCategoryIcons('chart-')
    expect(byKey.every((o) => o.key.includes('chart-') || o.label.toLowerCase().includes('chart-'))).toBe(true)
    expect(byKey.length).toBeGreaterThan(0)

    expect(filterCategoryIcons('zzz-no-such-icon')).toEqual([])
    expect(filterCategoryIcons('').length).toBe(CATEGORY_ICON_OPTIONS.length)
  })
})
