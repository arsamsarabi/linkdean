import type { Category } from '@/schemas'

export type CatNode = { cat: Category; children: CatNode[] }

export type CategorySelectOption = { value: string; label: string }

export function canWriteCategory(cat: Category): boolean {
  return cat.access === 'owner' || cat.access === 'write'
}

export function writableCategories(cats: Category[]): Category[] {
  return cats.filter(canWriteCategory)
}

export function buildCategoryTree(cats: Category[]): CatNode[] {
  const map = new Map<string, CatNode>()
  cats.forEach((c) => map.set(c.id, { cat: c, children: [] }))
  const roots: CatNode[] = []
  for (const n of map.values()) {
    if (n.cat.parent_id && map.has(n.cat.parent_id)) map.get(n.cat.parent_id)!.children.push(n)
    else roots.push(n)
  }
  return roots
}

export function categoryPath(cat: Category, byId: Map<string, Category>): string {
  const parts = [cat.name]
  let parent = cat.parent_id
  let guard = 0
  while (parent) {
    guard += 1
    if (guard > 64) break
    const p = byId.get(parent)
    if (!p) break
    parts.push(p.name)
    parent = p.parent_id
  }
  return parts.reverse().join(' / ')
}

/** Flat options for MultiSelect — label is nested path. */
export function categorySelectData(cats: Category[]): CategorySelectOption[] {
  const byId = new Map(cats.map((c) => [c.id, c]))
  return cats
    .map((c) => ({
      value: c.id,
      label: categoryPath(c, byId) + (c.is_vault ? ' (vault)' : '') + (c.is_hidden ? ' (hidden)' : ''),
    }))
    .sort((a, b) => a.label.localeCompare(b.label))
}

/** Self + descendant ids — exclude when picking a new parent. */
export function categorySubtreeIds(cats: Category[], rootId: string): Set<string> {
  const children = new Map<string, string[]>()
  for (const c of cats) {
    if (!c.parent_id) continue
    const list = children.get(c.parent_id) ?? []
    list.push(c.id)
    children.set(c.parent_id, list)
  }
  const out = new Set<string>([rootId])
  const stack = [rootId]
  while (stack.length > 0) {
    const id = stack.pop()!
    for (const child of children.get(id) ?? []) {
      if (out.has(child)) continue
      out.add(child)
      stack.push(child)
    }
  }
  return out
}
