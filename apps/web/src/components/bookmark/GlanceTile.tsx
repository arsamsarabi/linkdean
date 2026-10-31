import { useState, type ReactElement } from 'react'
import { Badge, Group, Image, Text, Tooltip, UnstyledButton } from '@mantine/core'
import { IconExternalLink, IconLock, IconWorld } from '@tabler/icons-react'
import { ShareBadge } from '@/components/share-badge'
import { inviteeShareAccess } from '@/lib'
import { googleFavicon, hostname } from '@/services'
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
  owner_display_name?: string | null
}

type Props = {
  bookmark: GlanceTileItem
  viewerUserId?: string
}

export function GlanceTile({ bookmark: b, viewerUserId }: Props): ReactElement {
  const title = b.title || b.url
  const [iconFailed, setIconFailed] = useState(false)
  const iconSrc = !iconFailed ? b.favicon_url || googleFavicon(b.url) : undefined
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
    >
      <Group gap="sm" wrap="nowrap" align="center" className={styles.inner}>
        <div className={styles.iconWrap}>
          {iconSrc ? (
            <Image src={iconSrc} alt="" w={20} h={20} fit="contain" onError={() => setIconFailed(true)} />
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
            {showOutgoing ? <ShareBadge outgoing /> : null}
            {b.is_vault ? (
              <Tooltip label="Stored in vault">
                <Badge size="xs" variant="light" leftSection={<IconLock size={8} stroke={1.5} />}>
                  vault
                </Badge>
              </Tooltip>
            ) : null}
          </Group>
          <Text size="xs" c="dimmed" lineClamp={1}>
            {hostname(b.url)}
          </Text>
        </div>
        <IconExternalLink size={14} stroke={1.5} className={styles.external} aria-hidden />
      </Group>
    </UnstyledButton>
  )
}
