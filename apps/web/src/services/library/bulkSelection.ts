/** Writable bookmark ids from a visible list (RO shares excluded). */
export function writableIds(
  items: ReadonlyArray<{ id: string; can_write?: boolean }>
): string[] {
  return items.filter((b) => b.can_write !== false).map((b) => b.id)
}

export function toggleId(selected: ReadonlySet<string>, id: string): Set<string> {
  const next = new Set(selected)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  return next
}

/** Union tags onto existing (trim, dedupe, case-sensitive). */
export function unionTags(existing: readonly string[], add: readonly string[]): string[] {
  const out = [...existing]
  const seen = new Set(existing.map((t) => t.trim()).filter(Boolean))
  for (const raw of add) {
    const t = raw.trim()
    if (!t || seen.has(t)) continue
    seen.add(t)
    out.push(t)
  }
  return out
}
