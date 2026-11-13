import { useState, type ReactElement } from 'react'
import { Tooltip, UnstyledButton } from '@mantine/core'
import { IconWorld } from '@tabler/icons-react'
import { useRecordBookmarkOpen } from '@/hooks'
import { googleFavicon } from '@/services'
import { ApiImage } from './ApiImage'
import type { GlanceTileItem } from './GlanceTile'
import styles from './IconTile.module.css'

type Props = {
  bookmark: GlanceTileItem
  /** Added to the tooltip, such as "12 opens". */
  meta?: string | null
  /** Small number in the corner, for a ranked section. */
  rank?: number | null
  /** Marks a bookmark saved in the last day. */
  fresh?: boolean
}

/** A dashboard bookmark shown as its site icon alone, titled by a tooltip. */
export function IconTile({ bookmark: b, meta, rank, fresh = false }: Props): ReactElement {
  const title = b.title.trim() || 'Untitled'
  const [iconFailed, setIconFailed] = useState(false)
  const recordOpen = useRecordBookmarkOpen()
  const iconSrc = !iconFailed ? b.favicon_url || (b.is_vault ? undefined : googleFavicon(b.url)) : undefined

  return (
    <Tooltip label={meta ? `${title} · ${meta}` : title} openDelay={150}>
      <UnstyledButton
        component="a"
        href={b.url}
        target="_blank"
        rel="noreferrer"
        className={styles.tile}
        aria-label={`Open ${title}`}
        onClick={() => {
          if (!b.is_vault) recordOpen(b.id)
        }}
      >
        {iconSrc ? (
          <ApiImage src={iconSrc} alt="" w={28} h={28} fit="contain" onError={() => setIconFailed(true)} />
        ) : (
          <IconWorld size={24} stroke={1.5} color="var(--mantine-color-dimmed)" />
        )}
        {rank ? <span className={styles.rank}>{rank}</span> : null}
        {fresh ? <span className={styles.fresh} aria-label="New" /> : null}
      </UnstyledButton>
    </Tooltip>
  )
}
