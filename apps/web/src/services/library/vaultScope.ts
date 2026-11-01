import type { Bookmark, Category } from '@/schemas'

/** Vault bookmarks, optionally from one vault. */
export function scopeBookmarks(items: Bookmark[] | undefined, vaultId: string | null): Bookmark[] {
  return (items ?? []).filter((b) => b.is_vault && (!vaultId || b.vault_id === vaultId))
}

/** Vault categories, optionally from one vault. */
export function scopeCategories(items: Category[] | undefined, vaultId: string | null): Category[] {
  return (items ?? []).filter((c) => c.is_vault && (!vaultId || c.vault_id === vaultId))
}

/** Categories that share a vault with at least one selected bookmark. */
export function categoriesForSelection(
  categories: Category[],
  items: readonly { id: string; vault_id?: string | null }[],
  selectedIds: ReadonlySet<string>
): Category[] {
  const vaults = new Set(items.filter((b) => selectedIds.has(b.id)).map((b) => b.vault_id))
  return categories.filter((c) => vaults.has(c.vault_id))
}

/** Case-insensitive match against a decrypted bookmark. */
export function matchesVaultQuery(item: Bookmark, query: string): boolean {
  const q = query.trim().toLowerCase()
  if (!q) return true
  return [item.title, item.url, item.description ?? '', ...item.tags].some((value) => value.toLowerCase().includes(q))
}
