import { type CSSProperties, type ReactElement, type ReactNode } from 'react'
import { SimpleGrid, Text, UnstyledButton } from '@mantine/core'
import { IconBookmarkPlus, IconBookmarks, IconEyeCheck, IconMail, IconStar } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { buildLibrarySearch, type LibrarySearch } from '@/lib'
import type { DashboardStats } from '@/schemas'
import styles from './DashboardHero.module.css'

type Stat = {
  label: string
  value: number
  color: string
  icon: ReactNode
  /** Where the tile leads in the library; none for a count the library cannot show. */
  search?: LibrarySearch
}

function StatTile({ label, value, color, icon, search }: Stat): ReactElement {
  const style = { '--stat-accent': `var(--mantine-color-${color}-6)` } as CSSProperties
  const body = (
    <>
      <div className={styles.statIcon} aria-hidden>
        {icon}
      </div>
      <div>
        <div className={styles.statValue}>{value.toLocaleString()}</div>
        <Text size="xs" c="dimmed">
          {label}
        </Text>
      </div>
    </>
  )
  if (!search) {
    return (
      <div className={styles.stat} style={style}>
        {body}
      </div>
    )
  }
  return (
    <UnstyledButton
      className={styles.stat}
      style={style}
      renderRoot={(props) => <Link {...props} to="/library" search={search} />}
    >
      {body}
    </UnstyledButton>
  )
}

/** Your own library at a glance; each count opens the matching library view. */
export function DashboardStatsRow({ stats }: { stats: DashboardStats }): ReactElement {
  const items: Stat[] = [
    {
      label: 'Bookmarks',
      value: stats.total,
      color: 'violet',
      icon: <IconBookmarks size={18} stroke={1.6} />,
      search: buildLibrarySearch({}),
    },
    {
      label: 'Saved this week',
      value: stats.saved_this_week,
      color: 'ice',
      icon: <IconBookmarkPlus size={18} stroke={1.6} />,
      search: buildLibrarySearch({ since: '7d', sort: 'created_desc' }),
    },
    {
      label: 'Unread',
      value: stats.unread,
      color: 'orchid',
      icon: <IconMail size={18} stroke={1.6} />,
      search: buildLibrarySearch({ flag: ['unread'] }),
    },
    {
      label: 'Favorites',
      value: stats.favorites,
      color: 'yellow',
      icon: <IconStar size={18} stroke={1.6} />,
      search: buildLibrarySearch({ flag: ['favorite'] }),
    },
    {
      label: 'Opened in 24 hours',
      value: stats.opened_today,
      color: 'mauve',
      icon: <IconEyeCheck size={18} stroke={1.6} />,
    },
  ]
  return (
    <SimpleGrid cols={{ base: 2, sm: 3, lg: 5 }} spacing="sm">
      {items.map((s) => (
        <StatTile key={s.label} {...s} />
      ))}
    </SimpleGrid>
  )
}
