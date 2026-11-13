import { type ReactElement } from 'react'
import { Group, SimpleGrid } from '@mantine/core'
import { BookmarkCard, GlanceTile, IconTile } from '@/components'
import type { Bookmark, DashboardSectionDisplay } from '@/schemas'

const CARD_COLS = {
  medium: { base: 1, xs: 2, sm: 4, lg: 6, xl: 8 },
  large: { base: 1, sm: 2, lg: 3, xl: 4 },
} as const

type Props = {
  display: DashboardSectionDisplay
  bookmarks: Bookmark[]
  viewerUserId: string | undefined
}

/** A section's bookmarks in its display mode: icons, small tiles, or medium or large cards. */
export function DashboardSectionItems({ display, bookmarks, viewerUserId }: Props): ReactElement {
  if (display === 'icons') {
    return (
      <Group gap="sm">
        {bookmarks.map((b) => (
          <IconTile key={b.id} bookmark={b} />
        ))}
      </Group>
    )
  }
  if (display === 'small') {
    return (
      <SimpleGrid cols={{ base: 1, xs: 2, sm: 3, lg: 4, xl: 5 }} spacing="sm">
        {bookmarks.map((b) => (
          <GlanceTile key={b.id} bookmark={b} viewerUserId={viewerUserId} />
        ))}
      </SimpleGrid>
    )
  }
  return (
    <SimpleGrid cols={CARD_COLS[display]} spacing="sm">
      {bookmarks.map((b) => (
        <BookmarkCard
          key={b.id}
          bookmark={{ ...b, can_write: false }}
          viewerUserId={viewerUserId}
          onEdit={() => undefined}
          onDelete={() => undefined}
        />
      ))}
    </SimpleGrid>
  )
}
