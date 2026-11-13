import { useState, type ReactElement } from 'react'
import { Badge, Group, Text, Tooltip, UnstyledButton } from '@mantine/core'
import { IconExternalLink, IconLock, IconWorld } from '@tabler/icons-react'
import { ShareBadge } from '@/components/share-badge'
import { useRecordBookmarkOpen } from '@/hooks'
import { inviteeShareAccess } from '@/lib'
import { googleFavicon } from '@/services'
import { ApiImage } from './ApiImage'
import styles from './GlanceTile.module.css'

export type GlanceTileItem = {
  id: string
  url: string
  title: string
  favicon_url?: string | null
  is_vault: boolean
  owner_user_id?: string
  can_write?: boolean
  shared_out?: boolean
  shared_with?: string[]
  owner_display_name?: string | null
}

type Props = {
  bookmark: GlanceTileItem
  viewerUserId?: string
}

export function GlanceTile({ bookmark: b, viewerUserId }: Props): ReactElement {
  const title = b.title.trim() || 'Untitled'
  const [iconFailed, setIconFailed] = useState(false)
  const recordOpen = useRecordBookmarkOpen()
  const iconSrc = !iconFailed ? b.favicon_url || (b.is_vault ? undefined : googleFavicon(b.url)) : undefined
  const share = inviteeShareAccess(b.owner_user_id, b.can_write, viewerUserId)
  const showOutgoing = !share && !!b.shared_out

  return (
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
      <Group gap="sm" wrap="nowrap" align="center" className={styles.inner}>
        <div className={styles.iconWrap}>
          {iconSrc ? (
            <ApiImage src={iconSrc} alt="" w={20} h={20} fit="contain" onError={() => setIconFailed(true)} />
          ) : (
            <IconWorld size={18} stroke={1.5} color="var(--mantine-color-dimmed)" />
          )}
        </div>
        <div className={styles.text}>
          <Group gap={6} wrap="nowrap" align="center">
            <Text fw={600} size="sm" lineClamp={1} className={styles.title}>
              {title}
            </Text>
            {share ? <ShareBadge access={share} ownerLabel={b.owner_display_name} /> : null}
            {showOutgoing ? <ShareBadge outgoing sharedWith={b.shared_with} /> : null}
            {b.is_vault ? (
              <Tooltip label="Stored in vault">
                <Badge size="xs" variant="light" leftSection={<IconLock size={8} stroke={1.5} />}>
                  vault
                </Badge>
              </Tooltip>
            ) : null}
          </Group>
          <Text size="xs" c="dimmed" lineClamp={1} title={b.url}>
            {b.url}
          </Text>
        </div>
        <IconExternalLink size={14} stroke={1.5} className={styles.external} aria-hidden />
      </Group>
    </UnstyledButton>
  )
}
