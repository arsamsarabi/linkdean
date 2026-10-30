import { useEffect, useMemo, useState } from 'react'
import { Link } from '@tanstack/react-router'
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
import { bookmarkPreviewSrc, type CategoryRef } from '../api'

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
}

function hostname(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, '')
  } catch {
    return url
  }
}

function googleFavicon(url: string) {
  const host = hostname(url)
  if (!host || host === url) return undefined
  return `https://www.google.com/s2/favicons?domain=${encodeURIComponent(host)}&sz=64`
}

function PreviewFallback({
  url,
  faviconUrl,
}: {
  url: string
  faviconUrl?: string | null
}) {
  const [iconFailed, setIconFailed] = useState(false)
  const iconSrc = !iconFailed ? faviconUrl || googleFavicon(url) : undefined

  return (
    <Group
      justify="center"
      align="center"
      h="100%"
      gap="sm"
      style={{
        background:
          'linear-gradient(145deg, var(--mantine-color-harbor-1), var(--mantine-color-mauve-1))',
      }}
    >
      {iconSrc ? (
        <Image
          src={iconSrc}
          alt=""
          w={32}
          h={32}
          fit="contain"
          onError={() => setIconFailed(true)}
        />
      ) : (
        <IconWorld size={28} stroke={1.5} color="var(--mantine-color-dimmed)" />
      )}
      <Text size="sm" c="dimmed" fw={600}>
        {hostname(url)}
      </Text>
    </Group>
  )
}

export function BookmarkCard({
  bookmark: b,
  highlightQuery,
  onEdit,
  onDelete,
}: {
  bookmark: BookmarkCardItem
  highlightQuery?: string
  onEdit: (id: string) => void
  onDelete: (id: string) => void
}) {
  const title = b.title || b.url
  const candidates = useMemo(() => {
    const list: string[] = []
    const primary = bookmarkPreviewSrc(b)
    if (primary) list.push(primary)
    if (b.has_preview && b.preview_image_url && b.preview_image_url !== primary) {
      list.push(b.preview_image_url)
    }
    return list
  }, [b])

  const [srcIndex, setSrcIndex] = useState(0)
  const previewSrc = candidates[srcIndex]

  useEffect(() => {
    setSrcIndex(0)
  }, [b.id, b.has_preview, b.preview_image_url])

  const cats = b.categories || []
  const q = highlightQuery?.trim()

  return (
    <Card
      withBorder
      padding="sm"
      radius="md"
      h="100%"
      className="ld-card"
      style={{ display: 'flex', flexDirection: 'column' }}
    >
      <Card.Section>
        <UnstyledButton
          component="a"
          href={b.url}
          target="_blank"
          rel="noreferrer"
          style={{ display: 'block', width: '100%' }}
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

      <Stack gap={8} mt="md" style={{ flex: 1 }}>
        <Group justify="space-between" align="flex-start" wrap="nowrap" gap="xs">
          <UnstyledButton
            component="a"
            href={b.url}
            target="_blank"
            rel="noreferrer"
            style={{ flex: 1, minWidth: 0 }}
          >
            <Group gap={6} wrap="nowrap" align="flex-start">
              <IconExternalLink
                size={14}
                stroke={1.5}
                style={{ flexShrink: 0, marginTop: 3 }}
              />
              {q ? (
                <Highlight
                  highlight={q}
                  fw={600}
                  lineClamp={2}
                  style={{ lineHeight: 1.35 }}
                >
                  {title}
                </Highlight>
              ) : (
                <Text fw={600} lineClamp={2} style={{ lineHeight: 1.35 }}>
                  {title}
                </Text>
              )}
            </Group>
          </UnstyledButton>
          <Group gap={2} wrap="nowrap">
            <CopyButton value={b.url} timeout={1500}>
              {({ copied, copy }) => (
                <Tooltip label={copied ? 'Copied' : 'Copy URL'}>
                  <ActionIcon
                    variant="subtle"
                    color={copied ? 'green' : 'gray'}
                    aria-label="Copy URL"
                    onClick={copy}
                  >
                    {copied ? (
                      <IconCheck size={16} stroke={1.5} />
                    ) : (
                      <IconCopy size={16} stroke={1.5} />
                    )}
                  </ActionIcon>
                </Tooltip>
              )}
            </CopyButton>
            <Menu shadow="md" width={160} position="bottom-end" withinPortal>
              <Menu.Target>
                <Tooltip label="Actions">
                  <ActionIcon variant="subtle" color="gray" aria-label="Bookmark actions">
                    <IconDotsVertical size={16} stroke={1.5} />
                  </ActionIcon>
                </Tooltip>
              </Menu.Target>
              <Menu.Dropdown>
                <Menu.Item
                  leftSection={<IconPencil size={14} stroke={1.5} />}
                  onClick={() => onEdit(b.id)}
                >
                  Edit
                </Menu.Item>
                <Menu.Item
                  color="red"
                  leftSection={<IconTrash size={14} stroke={1.5} />}
                  onClick={() => onDelete(b.id)}
                >
                  Delete
                </Menu.Item>
              </Menu.Dropdown>
            </Menu>
          </Group>
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
                style={{ textDecoration: 'none' }}
              >
                <Pill
                  size="sm"
                  styles={{
                    root: {
                      cursor: 'pointer',
                      backgroundColor: 'var(--mantine-color-harbor-light)',
                      color: 'var(--mantine-color-harbor-filled)',
                    },
                  }}
                >
                  {c.path}
                </Pill>
              </Link>
            ))}
          </Group>
        ) : null}

        <Group gap={4} mt="auto" pt={4}>
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
