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
}

/** A dashboard bookmark shown as its site icon alone, titled by a tooltip. */
export function IconTile({ bookmark: b }: Props): ReactElement {
  const title = b.title.trim() || 'Untitled'
  const [iconFailed, setIconFailed] = useState(false)
  const recordOpen = useRecordBookmarkOpen()
  const iconSrc = !iconFailed ? b.favicon_url || (b.is_vault ? undefined : googleFavicon(b.url)) : undefined

  return (
    <Tooltip label={title} openDelay={150}>
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
      </UnstyledButton>
    </Tooltip>
  )
}
