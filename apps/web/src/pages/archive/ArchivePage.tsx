import { type ReactElement } from 'react'
import { Button, Group, Loader, SegmentedControl, Stack, Text, Title } from '@mantine/core'
import { IconArchive, IconDownload, IconExternalLink } from '@tabler/icons-react'
import { useNavigate, useParams, useSearch } from '@tanstack/react-router'
import { EmptyState, PageContainer, QueryErrorAlert } from '@/components'
import { useArchives, useBookmark } from '@/hooks'
import {
  ARCHIVE_FORMAT_LABELS,
  archiveFileUrl,
  archiveMarkdownUrl,
  formatArchiveDate,
  formatBytes,
  primaryArchiveFormat,
} from '@/lib'
import type { BookmarkArchive } from '@/schemas'
import styles from './ArchivePage.module.css'

/** Archived pages run with no scripts and an opaque origin; links open in a new tab. */
export const ARCHIVE_IFRAME_SANDBOX = 'allow-popups allow-popups-to-escape-sandbox'

/** `/archive/{id}`: a stored copy inside Linkdean, with the live page and downloads a click away. */
export function ArchivePage(): ReactElement {
  const { id } = useParams({ from: '/_app/archive/$id' })
  const search = useSearch({ strict: false }) as { format?: string }
  const navigate = useNavigate()
  const bookmark = useBookmark(id)
  const archives = useArchives(id)

  if (archives.isLoading || bookmark.isLoading) {
    return (
      <PageContainer>
        <Loader type="dots" />
      </PageContainer>
    )
  }
  if (archives.isError) {
    return (
      <PageContainer>
        <QueryErrorAlert
          error={archives.error}
          onRetry={() => void archives.refetch()}
          title="Archive failed to load"
        />
      </PageContainer>
    )
  }
  const stored = (archives.data ?? []).filter((a) => a.available)
  const primary = primaryArchiveFormat(stored.map((a) => a.format))
  const current = stored.find((a) => a.format === search.format) ?? stored.find((a) => a.format === primary)
  const title = bookmark.data?.title.trim() || 'Untitled'

  if (!current) {
    return (
      <PageContainer>
        <EmptyState
          icon={<IconArchive size={28} stroke={1.5} />}
          title="No archived copy"
          description="Archive this bookmark from its menu to keep a copy here."
        />
      </PageContainer>
    )
  }

  return (
    <div className={styles.page}>
      <Group justify="space-between" align="flex-start" gap="sm" className={styles.bar}>
        <Stack gap={2} className={styles.heading}>
          <Title order={4} lineClamp={1}>
            {title}
          </Title>
          <Text size="xs" c="dimmed">
            Archived {formatArchiveDate(current.captured_at)} · {formatBytes(current.bytes)}
          </Text>
        </Stack>
        <Group gap="xs">
          {stored.length > 1 ? (
            <SegmentedControl
              size="xs"
              value={current.format}
              data={stored.map((a) => ({ value: a.format, label: ARCHIVE_FORMAT_LABELS[a.format] }))}
              onChange={(format) => void navigate({ to: '/archive/$id', params: { id }, search: { format } })}
            />
          ) : null}
          {bookmark.data?.url ? (
            <Button
              size="xs"
              variant="default"
              component="a"
              href={bookmark.data.url}
              target="_blank"
              rel="noreferrer"
              leftSection={<IconExternalLink size={14} stroke={1.5} />}
            >
              Live page
            </Button>
          ) : null}
          <Button
            size="xs"
            variant="light"
            component="a"
            href={archiveFileUrl(id, current.format, true)}
            leftSection={<IconDownload size={14} stroke={1.5} />}
          >
            Download
          </Button>
          {current.format === 'html' ? (
            <Button
              size="xs"
              variant="light"
              component="a"
              href={archiveMarkdownUrl(id)}
              leftSection={<IconDownload size={14} stroke={1.5} />}
            >
              Markdown
            </Button>
          ) : null}
        </Group>
      </Group>
      <ArchiveBody bookmarkId={id} archive={current} title={title} />
    </div>
  )
}

function ArchiveBody({
  bookmarkId,
  archive: a,
  title,
}: {
  bookmarkId: string
  archive: BookmarkArchive
  title: string
}): ReactElement {
  const src = archiveFileUrl(bookmarkId, a.format)
  const type = (a.content_type ?? '').split(';')[0].trim()
  if (a.format === 'html') {
    return (
      <iframe
        className={styles.frame}
        title={`Archived copy: ${title}`}
        src={src}
        sandbox={ARCHIVE_IFRAME_SANDBOX}
        referrerPolicy="no-referrer"
      />
    )
  }
  if (a.format === 'pdf' || type === 'application/pdf') {
    // No sandbox: it would stop the browser's PDF viewer. The server sends PDFs as PDFs only.
    return <iframe className={styles.frame} title={`Archived PDF: ${title}`} src={src} />
  }
  if (type.startsWith('image/') && type !== 'image/svg+xml') {
    return <img className={styles.media} src={src} alt={title} />
  }
  if (type.startsWith('video/')) {
    return <video className={styles.media} src={src} controls />
  }
  if (type.startsWith('audio/')) {
    return <audio src={src} controls />
  }
  return (
    <Stack align="center" gap="sm" py="xl">
      <Text size="sm" c="dimmed">
        This file can't be shown here. Download it to open it.
      </Text>
      <Button component="a" href={archiveFileUrl(bookmarkId, a.format, true)} leftSection={<IconDownload size={14} />}>
        Download
      </Button>
    </Stack>
  )
}
