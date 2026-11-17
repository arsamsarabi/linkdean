import { useState, type ReactElement } from 'react'
import { AspectRatio, Group, Text, UnstyledButton } from '@mantine/core'
import { IconWorld } from '@tabler/icons-react'
import { hostname } from '@/services'
import { ApiImage } from './ApiImage'
import styles from './BookmarkCard.module.css'

function PreviewFallback({ url, faviconUrl }: { url: string; faviconUrl?: string | null }): ReactElement {
  const [iconFailed, setIconFailed] = useState(false)
  const iconSrc = !iconFailed ? faviconUrl || undefined : undefined

  return (
    <Group justify="center" align="center" h="100%" gap="sm" className={styles.previewFallback}>
      {iconSrc ? (
        <ApiImage src={iconSrc} alt="" w={32} h={32} fit="contain" onError={() => setIconFailed(true)} />
      ) : (
        <IconWorld size={28} stroke={1.5} color="var(--mantine-color-dimmed)" />
      )}
      <Text size="sm" c="dimmed" fw={600}>
        {hostname(url)}
      </Text>
    </Group>
  )
}

/** A site icon for compact rows: the cached favicon, else a globe. */
export function BookmarkFavicon({
  faviconUrl,
  size = 20,
}: {
  faviconUrl?: string | null
  size?: number
}): ReactElement {
  const [iconFailed, setIconFailed] = useState(false)
  const iconSrc = !iconFailed ? faviconUrl || undefined : undefined
  return iconSrc ? (
    <ApiImage src={iconSrc} alt="" w={size} h={size} fit="contain" onError={() => setIconFailed(true)} />
  ) : (
    <IconWorld size={size} stroke={1.5} color="var(--mantine-color-dimmed)" />
  )
}

type Props = {
  url: string
  title: string
  previewSrc: string | undefined
  faviconUrl?: string | null
  onPreviewError: () => void
}

export function BookmarkCardPreview({ url, title, previewSrc, faviconUrl, onPreviewError }: Props): ReactElement {
  return (
    <UnstyledButton
      component="a"
      href={url}
      target="_blank"
      rel="noreferrer"
      className={styles.previewLink}
      aria-label={`Open ${title}`}
    >
      <AspectRatio ratio={16 / 9}>
        {previewSrc ? (
          <ApiImage src={previewSrc} alt="" h="100%" fit="cover" onError={onPreviewError} />
        ) : (
          <PreviewFallback url={url} faviconUrl={faviconUrl} />
        )}
      </AspectRatio>
    </UnstyledButton>
  )
}
