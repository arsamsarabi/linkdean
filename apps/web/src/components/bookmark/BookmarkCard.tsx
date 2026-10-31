import { useMemo, useState, type ReactElement } from 'react'
import {
  ActionIcon,
  AspectRatio,
  Badge,
  Card,
  CopyButton,
  Group,
  Highlight,
  Image,
  Menu,
  Pill,
  Spoiler,
  Stack,
  Text,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import {
  IconCheck,
  IconCopy,
  IconDotsVertical,
  IconExternalLink,
  IconLock,
  IconPencil,
  IconTrash,
  IconWorld,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { ShareBadge } from '@/components/share-badge'
import { inviteeShareAccess } from '@/lib'
import type { CategoryRef } from '@/schemas'
import { googleFavicon, hostname, previewSrcCandidates } from '@/services'
import styles from './BookmarkCard.module.css'

export type BookmarkCardItem = {
  id: string
  url: string
  title: string
  description?: string | null
  favicon_url?: string | null
  preview_image_url?: string | null
  has_preview?: boolean
  is_vault: boolean
  tags: string[]
  categories?: CategoryRef[]
  owner_user_id?: string
  /** When false, edit/delete stay hidden (read-only share). Default true for legacy callers. */
  can_write?: boolean
  /** Owner has shared a folder this bookmark lives in. */
  shared_out?: boolean
}

type PreviewFallbackProps = {
  url: string
  faviconUrl?: string | null
}

function PreviewFallback({ url, faviconUrl }: PreviewFallbackProps): ReactElement {
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
  bookmark: BookmarkCardItem
  highlightQuery?: string
  /** Current user — used to show Shared · RO/RW when item is from another owner. */
  viewerUserId?: string
  onEdit: (id: string) => void
  onDelete: (id: string) => void
}

export function BookmarkCard({
  bookmark: b,
  highlightQuery,
  viewerUserId,
  onEdit,
  onDelete,
}: Props): ReactElement {
  const title = b.title || b.url
  const canWrite = b.can_write !== false
  const share = inviteeShareAccess(b.owner_user_id, b.can_write, viewerUserId)
  const showOutgoing = !share && !!b.shared_out
  const candidates = useMemo(() => previewSrcCandidates(b), [b])

  const previewKey = `${b.id}:${String(b.has_preview)}:${b.preview_image_url ?? ''}`
  const [srcIndex, setSrcIndex] = useState(0)
  const [srcForKey, setSrcForKey] = useState(previewKey)
  if (srcForKey !== previewKey) {
    setSrcForKey(previewKey)
    setSrcIndex(0)
  }
  const previewSrc = candidates[srcIndex]

  const cats = b.categories || []
  const q = highlightQuery?.trim()

  return (
    <Card withBorder padding="sm" radius="md" h="100%" className={styles.card}>
      <Card.Section>
        <UnstyledButton
          component="a"
          href={b.url}
          target="_blank"
          rel="noreferrer"
          className={styles.previewLink}
          aria-label={`Open ${title}`}
        >
          <AspectRatio ratio={16 / 9}>
            {previewSrc ? (
              <Image
                src={previewSrc}
                alt=""
                h="100%"
                fit="cover"
                onError={() => {
                  setSrcIndex((i) => i + 1)
                }}
              />
            ) : (
              <PreviewFallback url={b.url} faviconUrl={b.favicon_url} />
            )}
          </AspectRatio>
        </UnstyledButton>
      </Card.Section>

      <Stack gap={8} mt="md" className={styles.body}>
        <Group justify="space-between" align="flex-start" wrap="nowrap" gap="xs">
          <UnstyledButton component="a" href={b.url} target="_blank" rel="noreferrer" className={styles.titleLink}>
            <Group gap={6} wrap="nowrap" align="flex-start">
              <IconExternalLink size={14} stroke={1.5} className={styles.externalIcon} />
              {q ? (
                <Highlight highlight={q} fw={600} lineClamp={2} className={styles.titleText}>
                  {title}
                </Highlight>
              ) : (
                <Text fw={600} lineClamp={2} className={styles.titleText}>
                  {title}
                </Text>
              )}
            </Group>
          </UnstyledButton>
          <Menu shadow="md" width={160} position="bottom-end" withinPortal>
            <Menu.Target>
              <Tooltip label="Actions">
                <ActionIcon variant="subtle" color="gray" aria-label="Bookmark actions">
                  <IconDotsVertical size={16} stroke={1.5} />
                </ActionIcon>
              </Tooltip>
            </Menu.Target>
            <Menu.Dropdown>
              <CopyButton value={b.url} timeout={1500}>
                {({ copied, copy }) => (
                  <Menu.Item
                    leftSection={
                      copied ? <IconCheck size={14} stroke={1.5} /> : <IconCopy size={14} stroke={1.5} />
                    }
                    onClick={copy}
                  >
                    {copied ? 'Copied' : 'Copy URL'}
                  </Menu.Item>
                )}
              </CopyButton>
              {canWrite ? (
                <>
                  <Menu.Item leftSection={<IconPencil size={14} stroke={1.5} />} onClick={() => onEdit(b.id)}>
                    Edit
                  </Menu.Item>
                  <Menu.Item
                    color="red"
                    leftSection={<IconTrash size={14} stroke={1.5} />}
                    onClick={() => onDelete(b.id)}
                  >
                    Delete
                  </Menu.Item>
                </>
              ) : null}
            </Menu.Dropdown>
          </Menu>
        </Group>

        <Text size="xs" c="dimmed" lineClamp={1}>
          {hostname(b.url)}
        </Text>

        {b.description ? (
          <Spoiler maxHeight={40} showLabel="More" hideLabel="Less">
            <Text size="sm" c="dimmed">
              {b.description}
            </Text>
          </Spoiler>
        ) : null}

        {cats.length > 0 ? (
          <Group gap={4}>
            {cats.map((c) => (
              <Link
                key={c.id}
                to="/library"
                search={{ category: c.id }}
                onClick={(e) => e.stopPropagation()}
                className={styles.categoryLink}
              >
                <Pill size="sm" className={styles.categoryPill}>
                  {c.path}
                </Pill>
              </Link>
            ))}
          </Group>
        ) : null}

        <Group gap={4} mt="auto" pt={4}>
          {share ? <ShareBadge access={share} size="sm" /> : null}
          {showOutgoing ? <ShareBadge outgoing size="sm" /> : null}
          {b.is_vault ? (
            <Tooltip label="Stored in vault">
              <Badge size="sm" leftSection={<IconLock size={10} stroke={1.5} />}>
                vault
              </Badge>
            </Tooltip>
          ) : null}
          {(b.tags || []).map((t) => (
            <Badge key={t} variant="outline" size="sm">
              {t}
            </Badge>
          ))}
        </Group>
      </Stack>
    </Card>
  )
}
