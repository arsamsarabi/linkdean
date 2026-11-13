import { useState, type ReactElement } from 'react'
import { Group, SimpleGrid, Text } from '@mantine/core'
import { GlanceTile, IconTile } from '@/components'
import type { Bookmark, DashboardSectionDisplay, DashboardSectionKind } from '@/schemas'
import { itemMeta } from '@/services'
import { DashboardCard } from './DashboardCard'
import styles from './DashboardSectionItems.module.css'

// Columns follow the section's own width (container queries), so half and third sections fit.
const SMALL_COLS = { base: 1, '440px': 2, '760px': 3, '1040px': 4 }
const CARD_COLS = {
  medium: { base: 1, '360px': 2, '560px': 3, '760px': 4, '1100px': 6 },
  large: { base: 1, '520px': 2, '820px': 3, '1100px': 4 },
} as const

export type SectionItem = {
  bookmark: Bookmark
  open_count?: number | null
  last_opened_at?: string | null
}

type Props = {
  kind: DashboardSectionKind
  display: DashboardSectionDisplay
  items: SectionItem[]
  viewerUserId: string | undefined
}

/**
 * A section's bookmarks in its display mode: icons, small tiles, or medium or large cards.
 * Each one carries what its section adds: rank and opens, when it was opened, or when saved.
 * Every link sits inside a `data-dash-tile` box so the keyboard can move between them, and the
 * first nine carry the digit that opens them (shown on the highlighted section only).
 */
function Keycap({ index }: { index: number }): ReactElement | null {
  if (index > 8) return null
  return (
    <kbd className={styles.keycap} aria-hidden>
      {index + 1}
    </kbd>
  )
}

export function DashboardSectionItems({ kind, display, items, viewerUserId }: Props): ReactElement {
  // Fixed at mount: "2h ago" need not tick while the page is open.
  const [now] = useState(() => Date.now())
  const metas = items.map((item, i) => itemMeta({ kind }, item, i, now))

  if (display === 'icons') {
    return (
      <Group gap="md">
        {items.map((item, i) => (
          <div key={item.bookmark.id} data-dash-tile className={styles.tile}>
            {metas[i]!.rank ? null : <Keycap index={i} />}
            <IconTile bookmark={item.bookmark} meta={metas[i]!.text} rank={metas[i]!.rank} fresh={metas[i]!.fresh} />
          </div>
        ))}
      </Group>
    )
  }
  if (display === 'small') {
    return (
      <div className={styles.container}>
        {/* Room between rows for the key numbers that sit on each tile's corner. */}
        <SimpleGrid type="container" cols={SMALL_COLS} spacing="md" verticalSpacing="md">
          {items.map((item, i) => (
            <Group key={item.bookmark.id} gap="xs" wrap="nowrap" align="center">
              {metas[i]!.rank ? (
                <Text
                  className={styles.rankKey}
                  fw={700}
                  size="sm"
                  c="dimmed"
                  w={22}
                  ta="center"
                  style={{ flexShrink: 0 }}
                  aria-hidden
                >
                  {metas[i]!.rank}
                </Text>
              ) : null}
              {/* The ring and the key go round the bookmark only, not its rank. */}
              <div data-dash-tile className={styles.tile} style={{ flex: 1, minWidth: 0 }}>
                {metas[i]!.rank ? null : <Keycap index={i} />}
                <GlanceTile
                  bookmark={item.bookmark}
                  viewerUserId={viewerUserId}
                  meta={metas[i]!.text}
                  fresh={metas[i]!.fresh}
                />
              </div>
            </Group>
          ))}
        </SimpleGrid>
      </div>
    )
  }
  return (
    <div className={styles.container}>
      <SimpleGrid type="container" cols={CARD_COLS[display]} spacing="md">
        {items.map((item, i) => {
          const meta = metas[i]!
          return (
            <div key={item.bookmark.id} data-dash-tile className={styles.tile}>
              <Keycap index={i} />
              <DashboardCard
                bookmark={item.bookmark}
                kind={kind}
                meta={meta}
                viewerUserId={viewerUserId}
                size={display}
              />
            </div>
          )
        })}
      </SimpleGrid>
    </div>
  )
}
