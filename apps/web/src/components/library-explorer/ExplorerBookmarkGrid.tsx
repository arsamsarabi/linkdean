import { type ReactElement } from 'react'
import { SimpleGrid } from '@mantine/core'
import { BookmarkCard } from '@/components/bookmark'
import type { LibraryListItem } from '@/services'

type Props = {
  items: LibraryListItem[]
  highlightQuery?: string
  viewerUserId?: string
  onEdit: (id: string) => void
  onDelete: (id: string) => void
  onFlags?: (id: string, patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>) => void
  selectMode?: boolean
  selectedIds?: ReadonlySet<string>
  onToggleSelect?: (id: string) => void
}

export function ExplorerBookmarkGrid({
  items,
  highlightQuery,
  viewerUserId,
  onEdit,
  onDelete,
  onFlags,
  selectMode,
  selectedIds,
  onToggleSelect,
}: Props): ReactElement {
  return (
    <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="lg">
      {items.map((b) => (
        <BookmarkCard
          key={b.id}
          bookmark={b}
          highlightQuery={highlightQuery}
          viewerUserId={viewerUserId}
          onEdit={onEdit}
          onDelete={onDelete}
          onFlags={onFlags}
          selectMode={selectMode}
          selected={selectedIds?.has(b.id)}
          onToggleSelect={onToggleSelect}
        />
      ))}
    </SimpleGrid>
  )
}
