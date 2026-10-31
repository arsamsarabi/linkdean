import { useState, type ReactElement } from 'react'
import { Badge, Group, Image, Text, Tooltip, UnstyledButton } from '@mantine/core'
import { IconExternalLink, IconLock, IconWorld } from '@tabler/icons-react'
import { googleFavicon, hostname } from '@/services'
import styles from './GlanceTile.module.css'

export type GlanceTileItem = {
  id: string
  url: string
  title: string
  favicon_url?: string | null
  is_vault: boolean
}

type Props = {
  bookmark: GlanceTileItem
}

export function GlanceTile({ bookmark: b }: Props): ReactElement {
  const title = b.title || b.url
  const [iconFailed, setIconFailed] = useState(false)
  const iconSrc = !iconFailed ? b.favicon_url || googleFavicon(b.url) : undefined

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
