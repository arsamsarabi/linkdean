import { describe, expect, it } from 'vitest'
import type { Category } from '@/schemas'
import { buildCategoryTree, categoryPath, categorySelectData } from './categories'

const cats: Category[] = [
  { id: '1', parent_id: null, name: 'Root', is_hidden: false, is_vault: false, position: 0 },
  { id: '2', parent_id: '1', name: 'Child', is_hidden: true, is_vault: true, position: 1 },
]

describe('buildCategoryTree', () => {
  it('nests children under parent', () => {
    const tree = buildCategoryTree(cats)
    expect(tree).toHaveLength(1)
    expect(tree[0]?.cat.id).toBe('1')
    expect(tree[0]?.children[0]?.cat.id).toBe('2')
  })
})

describe('categoryPath', () => {
  it('joins ancestor names', () => {
    const byId = new Map(cats.map((c) => [c.id, c]))
    expect(categoryPath(cats[1]!, byId)).toBe('Root / Child')
  })
})

describe('categorySelectData', () => {
  it('labels vault and hidden', () => {
    const data = categorySelectData(cats)
    expect(data.find((d) => d.value === '2')?.label).toContain('(vault)')
    expect(data.find((d) => d.value === '2')?.label).toContain('(hidden)')
  })
})
