import { type CSSProperties, type ReactElement, type ReactNode } from 'react'
import { SimpleGrid, Text } from '@mantine/core'
import { IconBookmarkPlus, IconBookmarks, IconEyeCheck, IconMail, IconStar } from '@tabler/icons-react'
import type { DashboardStats } from '@/services'
import styles from './DashboardHero.module.css'

type Stat = { label: string; value: number; color: string; icon: ReactNode }

function StatTile({ label, value, color, icon }: Stat): ReactElement {
  const style = { '--stat-accent': `var(--mantine-color-${color}-6)` } as CSSProperties
  return (
    <div className={styles.stat} style={style}>
      <div className={styles.statIcon} aria-hidden>
        {icon}
      </div>
      <div>
        <div className={styles.statValue}>{value.toLocaleString()}</div>
        <Text size="xs" c="dimmed">
          {label}
        </Text>
      </div>
    </div>
  )
}

/** This dashboard at a glance: counts over the distinct bookmarks it shows. */
export function DashboardStatsRow({ stats }: { stats: DashboardStats }): ReactElement {
  const items: Stat[] = [
    {
      label: stats.bookmarks === 1 ? 'Bookmark here' : 'Bookmarks here',
      value: stats.bookmarks,
      color: 'violet',
      icon: <IconBookmarks size={18} stroke={1.6} />,
    },
    {
      label: 'Added this week',
      value: stats.addedThisWeek,
      color: 'ice',
      icon: <IconBookmarkPlus size={18} stroke={1.6} />,
    },
    { label: 'Unread', value: stats.unread, color: 'orchid', icon: <IconMail size={18} stroke={1.6} /> },
    { label: 'Favorites', value: stats.favorites, color: 'yellow', icon: <IconStar size={18} stroke={1.6} /> },
    {
      label: 'Opened in 24 hours',
      value: stats.openedToday,
      color: 'green',
      icon: <IconEyeCheck size={18} stroke={1.6} />,
    },
  ]
  return (
    <SimpleGrid cols={{ base: 2, sm: 3, lg: 5 }} spacing="sm" aria-label="This dashboard at a glance">
      {items.map((s) => (
        <StatTile key={s.label} {...s} />
      ))}
    </SimpleGrid>
  )
}
