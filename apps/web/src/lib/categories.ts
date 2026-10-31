import type { Category } from '../schemas'

export type CatNode = { cat: Category; children: CatNode[] }

export type CategorySelectOption = { value: string; label: string }

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
