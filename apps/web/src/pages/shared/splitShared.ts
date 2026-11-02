import type { Category, DashboardSummary } from '@/schemas'

export type SharedItem = {
  key: string
  kind: 'folder' | 'dashboard'
  id: string
  title: string
  /** Shared with me: who owns it. Shared by me: who it is shared with. */
  people: string
  /** Shared with me only: what I may do. */
  access?: 'read' | 'write'
}

function sharedWith(names: string[]): string {
  return names.length ? `with ${names.join(', ')}` : 'shared'
}

/** Split folders and dashboards into what others share with me and what I share. */
export function splitShared(
  categories: Category[],
  dashboards: DashboardSummary[]
): { withMe: SharedItem[]; byMe: SharedItem[] } {
  const live = categories.filter((c) => !c.deleted_at && !c.is_vault)
  const incoming = new Set(live.filter((c) => c.access !== 'owner').map((c) => c.id))
  const outgoing = new Set(live.filter((c) => c.access === 'owner' && c.shared_out).map((c) => c.id))
  const withMe: SharedItem[] = []
  const byMe: SharedItem[] = []
  for (const c of live) {
    const base = { key: `c:${c.id}`, kind: 'folder' as const, id: c.id, title: c.name }
    if (c.access !== 'owner') {
      // A subfolder of a shared folder comes with it; list only the top one.
      if (c.parent_id && incoming.has(c.parent_id)) continue
      withMe.push({ ...base, people: `from ${c.owner_display_name || 'someone'}`, access: c.access })
    } else if (c.shared_out) {
      // Shared only through its parent: the parent's entry covers it.
      if (c.parent_id && outgoing.has(c.parent_id)) continue
      byMe.push({ ...base, people: sharedWith(c.shared_with) })
    }
  }
  for (const d of dashboards) {
    if (d.deleted_at) continue
    const base = { key: `d:${d.id}`, kind: 'dashboard' as const, id: d.id, title: d.title }
    if (d.access !== 'owner') {
      withMe.push({ ...base, people: `from ${d.owner_display_name || 'someone'}`, access: d.access })
    } else if (d.shared_out) {
      byMe.push({ ...base, people: sharedWith(d.shared_with) })
    }
  }
  const byTitle = (a: SharedItem, b: SharedItem) => a.title.localeCompare(b.title)
  return { withMe: withMe.sort(byTitle), byMe: byMe.sort(byTitle) }
}
