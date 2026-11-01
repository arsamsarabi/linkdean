import { useState, type ReactElement } from 'react'
import { AspectRatio, Group, Image, Text, UnstyledButton } from '@mantine/core'
import { IconWorld } from '@tabler/icons-react'
import { googleFavicon, hostname } from '@/services'
import styles from './BookmarkCard.module.css'

type FallbackProps = {
  url: string
  faviconUrl?: string | null
}

function PreviewFallback({ url, faviconUrl }: FallbackProps): ReactElement {
  const [iconFailed, setIconFailed] = useState(false)
  const iconSrc = !iconFailed ? faviconUrl || googleFavicon(url) : undefined

  return (
    <Group justify="center" align="center" h="100%" gap="sm" className={styles.previewFallback}>
      {iconSrc ? (
        <Image src={iconSrc} alt="" w={32} h={32} fit="contain" onError={() => setIconFailed(true)} />
      ) : (
        <IconWorld size={28} stroke={1.5} color="var(--mantine-color-dimmed)" />
      )}
      <Text size="sm" c="dimmed" fw={600}>
        {hostname(url)}
      </Text>
    </Group>
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
          <Image src={previewSrc} alt="" h="100%" fit="cover" onError={onPreviewError} />
        ) : (
          <PreviewFallback url={url} faviconUrl={faviconUrl} />
        )}
      </AspectRatio>
    </UnstyledButton>
  )
}
