import { useMemo, useState, type ReactElement } from 'react'
import {
  ActionIcon,
  AspectRatio,
  Badge,
  Card,
  Checkbox,
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
  IconBookmark,
  IconBookmarkFilled,
  IconCheck,
  IconCopy,
  IconDotsVertical,
  IconExternalLink,
  IconLock,
  IconPencil,
  IconPin,
  IconPinFilled,
  IconStar,
  IconStarFilled,
  IconTrash,
  IconWorld,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { ShareBadge } from '@/components/share-badge'
import { inviteeShareAccess } from '@/lib'
import type { CategoryRef } from '@/schemas'
import { FOLDER_UNCATEGORIZED, googleFavicon, hostname, previewSrcCandidates } from '@/services'
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
  /** Owner-only: invitee labels for outgoing share tooltip. */
  shared_with?: string[]
  /** Invitee-only: owner's display name for share badge. */
  owner_display_name?: string | null
  is_favorite?: boolean
  is_unread?: boolean
  is_pinned?: boolean
  deleted_at?: string | null
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
  /** Toggle per-viewer flags (favorite / unread / pin). */
  onFlags?: (
    id: string,
    patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>
  ) => void
  selectMode?: boolean
  selected?: boolean
  onToggleSelect?: (id: string) => void
}

export function BookmarkCard({
  bookmark: b,
  highlightQuery,
  viewerUserId,
  onEdit,
  onDelete,
  onFlags,
  selectMode = false,
  selected = false,
  onToggleSelect,
}: Props): ReactElement {
  const title = b.title || b.url
  const canWrite = b.can_write !== false
  const selectable = selectMode && canWrite && !!onToggleSelect
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
  const pendingDelete = !!b.deleted_at
  const cardClass = [
    styles.card,
    selected ? styles.selected : '',
    selectMode && !canWrite ? styles.selectDisabled : '',
    pendingDelete ? styles.pendingDelete : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <Card withBorder padding="sm" radius="md" h="100%" className={cardClass}>
      {selectable ? (
        <Checkbox
          className={styles.selectCheck}
          checked={selected}
          onChange={() => onToggleSelect?.(b.id)}
          aria-label={`Select ${title}`}
          onClick={(e) => e.stopPropagation()}
        />
      ) : null}
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
          <UnstyledButton
            component="a"
            href={b.url}
            target="_blank"
            rel="noreferrer"
            className={styles.titleLink}
            onClick={() => {
              if (b.is_unread && onFlags) onFlags(b.id, { is_unread: false })
            }}
          >
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
          <Menu shadow="md" width={200} position="bottom-end" withinPortal>
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
              {onFlags ? (
                <>
                  <Menu.Item
                    leftSection={
                      b.is_favorite ? (
                        <IconStarFilled size={14} stroke={1.5} />
                      ) : (
                        <IconStar size={14} stroke={1.5} />
                      )
                    }
                    onClick={() => onFlags(b.id, { is_favorite: !b.is_favorite })}
                  >
                    {b.is_favorite ? 'Unfavorite' : 'Favorite'}
                  </Menu.Item>
                  <Menu.Item
                    leftSection={
                      b.is_unread ? (
                        <IconBookmarkFilled size={14} stroke={1.5} />
                      ) : (
                        <IconBookmark size={14} stroke={1.5} />
                      )
                    }
                    onClick={() => onFlags(b.id, { is_unread: !b.is_unread })}
                  >
                    {b.is_unread ? 'Mark read' : 'Reading list'}
                  </Menu.Item>
                  <Menu.Item
                    leftSection={
                      b.is_pinned ? <IconPinFilled size={14} stroke={1.5} /> : <IconPin size={14} stroke={1.5} />
                    }
                    onClick={() => onFlags(b.id, { is_pinned: !b.is_pinned })}
                  >
                    {b.is_pinned ? 'Unpin' : 'Pin to top'}
                  </Menu.Item>
                </>
              ) : null}
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
          <Group gap={4} className={styles.categoryRow}>
            {cats.map((c) => (
              <Tooltip key={c.id} label={c.path} openDelay={400}>
                <Link
                  to="/library"
                  search={{ category: c.id }}
                  onClick={(e) => e.stopPropagation()}
                  className={styles.categoryLink}
                >
                  <Pill size="sm" className={styles.categoryPill}>
                    {c.path}
                  </Pill>
                </Link>
              </Tooltip>
            ))}
          </Group>
        ) : (
          <Group gap={4} className={styles.categoryRow}>
            <Tooltip label="No category — open Unsorted in library" openDelay={400}>
              <Link
                to="/library"
                search={{ folder: FOLDER_UNCATEGORIZED, category: FOLDER_UNCATEGORIZED }}
                onClick={(e) => e.stopPropagation()}
                className={styles.categoryLink}
              >
                <Pill size="sm" className={styles.unsortedPill}>
                  Unsorted
                </Pill>
              </Link>
            </Tooltip>
          </Group>
        )}

        <Group gap={4} mt="auto" pt={4}>
          {share ? <ShareBadge access={share} ownerLabel={b.owner_display_name} size="sm" /> : null}
          {showOutgoing ? <ShareBadge outgoing sharedWith={b.shared_with} size="sm" /> : null}
          {pendingDelete ? (
            <Badge size="sm" variant="outline" color="red" tt="none">
              Pending delete
            </Badge>
          ) : null}
          {b.is_favorite ? (
            <Badge size="sm" variant="light" color="yellow" leftSection={<IconStarFilled size={10} stroke={1.5} />}>
              favorite
            </Badge>
          ) : null}
          {b.is_unread ? (
            <Badge size="sm" variant="light" color="blue" leftSection={<IconBookmarkFilled size={10} stroke={1.5} />}>
              unread
            </Badge>
          ) : null}
          {b.is_pinned ? (
            <Badge size="sm" variant="light" leftSection={<IconPinFilled size={10} stroke={1.5} />}>
              pinned
            </Badge>
          ) : null}
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
