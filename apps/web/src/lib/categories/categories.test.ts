import { describe, expect, it } from 'vitest'
import type { Category } from '@/schemas'
import {
  buildCategoryTree,
  canWriteCategory,
  categoryPath,
  categorySelectData,
  writableCategories,
} from './categories'

const cats: Category[] = [
  {
    id: '1',
    parent_id: null,
    name: 'Root',
    is_hidden: false,
    is_vault: false,
    position: 0,
    access: 'owner',
    shared_out: false,
  },
  {
    id: '2',
    parent_id: '1',
    name: 'Child',
    is_hidden: true,
    is_vault: true,
    position: 1,
    access: 'owner',
    shared_out: false,
  },
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

describe('canWriteCategory / writableCategories', () => {
  it('treats owner and write as writable', () => {
    expect(canWriteCategory({ ...cats[0]!, access: 'owner' })).toBe(true)
    expect(canWriteCategory({ ...cats[0]!, access: 'write' })).toBe(true)
    expect(canWriteCategory({ ...cats[0]!, access: 'read' })).toBe(false)
  })

  it('filters writable list', () => {
    const mixed: Category[] = [
      { ...cats[0]!, access: 'owner' },
      { ...cats[1]!, id: 'ro', access: 'read' },
    ]
    expect(writableCategories(mixed).map((c) => c.id)).toEqual(['1'])
  })
})
