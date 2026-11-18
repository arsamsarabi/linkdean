import { useState, type ReactElement } from 'react'
import { Anchor, Badge, Button, Group, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconArchive, IconDownload, IconEye, IconTrash } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { ConfirmModal } from '@/components/confirm-modal'
import { useArchives, useDeleteArchives } from '@/hooks'
import { ARCHIVE_FORMAT_LABELS, archiveFileUrl, archiveMarkdownUrl, formatArchiveDate, formatBytes } from '@/lib'
import type { BookmarkArchive } from '@/schemas'
import { ArchiveDialog } from './ArchiveDialog'

type Props = {
  bookmarkId: string
  canWrite: boolean
}

/** The bookmark drawer's Archive section: each copy's state, and what can be done with it. */
export function ArchiveSection({ bookmarkId, canWrite }: Props): ReactElement {
  const archives = useArchives(bookmarkId)
  const del = useDeleteArchives()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [confirmOpen, setConfirmOpen] = useState(false)
  const rows = archives.data ?? []
  const stored = rows.filter((a) => a.available)
  const newest =
    stored
      .map((a) => a.captured_at ?? '')
      .sort()
      .at(-1) || null

  async function onDelete(): Promise<void> {
    try {
      await del.mutateAsync(bookmarkId)
      setConfirmOpen(false)
      notifications.show({ message: 'Archive deleted' })
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Stack gap={6}>
      <Text size="sm" fw={500}>
        Archive
      </Text>
      {rows.length === 0 ? (
        <Text size="sm" c="dimmed">
          {archives.isLoading
            ? 'Loading…'
            : 'No copy kept yet. Archive the page to keep it if the site changes or goes away.'}
        </Text>
      ) : (
        rows.map((a) => <ArchiveRow key={a.format} bookmarkId={bookmarkId} archive={a} />)
      )}
      {canWrite ? (
        <Group gap="xs">
          <Button
            size="xs"
            variant="light"
            leftSection={<IconArchive size={14} stroke={1.5} />}
            onClick={() => setDialogOpen(true)}
          >
            {stored.length ? 'Archive again…' : 'Archive…'}
          </Button>
          {rows.length ? (
            <Button
              size="xs"
              variant="subtle"
              color="red"
              leftSection={<IconTrash size={14} stroke={1.5} />}
              onClick={() => setConfirmOpen(true)}
            >
              Delete archive
            </Button>
          ) : null}
        </Group>
      ) : null}
      <ArchiveDialog opened={dialogOpen} onClose={() => setDialogOpen(false)} ids={[bookmarkId]} replaces={newest} />
      <ConfirmModal
        opened={confirmOpen}
        onClose={() => setConfirmOpen(false)}
        onConfirm={() => void onDelete()}
        title="Delete archive?"
        body="Every archived copy of this bookmark is deleted. The bookmark stays."
        confirmLabel="Delete archive"
        loading={del.isPending}
      />
    </Stack>
  )
}

function ArchiveRow({ bookmarkId, archive: a }: { bookmarkId: string; archive: BookmarkArchive }): ReactElement {
  const pending = a.status === 'queued' || a.status === 'running'
  return (
    <Stack gap={2}>
      <Group gap="xs" wrap="wrap">
        <Text size="sm">{ARCHIVE_FORMAT_LABELS[a.format]}</Text>
        {pending ? (
          <Badge size="sm" variant="light" color="blue">
            {a.status === 'queued' ? 'Queued' : 'Archiving…'}
          </Badge>
        ) : null}
        {a.available ? (
          <Text size="xs" c="dimmed">
            {formatArchiveDate(a.captured_at)} · {formatBytes(a.bytes)}
          </Text>
        ) : null}
        {a.available ? (
          <Group gap={8}>
            <Anchor component={Link} to={`/archive/${bookmarkId}`} size="xs">
              <Group gap={2} component="span">
                <IconEye size={12} stroke={1.5} /> Open
              </Group>
            </Anchor>
            <Anchor href={archiveFileUrl(bookmarkId, a.format, true)} size="xs">
              <Group gap={2} component="span">
                <IconDownload size={12} stroke={1.5} /> Download
              </Group>
            </Anchor>
            {a.format === 'html' ? (
              <Anchor href={archiveMarkdownUrl(bookmarkId)} size="xs">
                <Group gap={2} component="span">
                  <IconDownload size={12} stroke={1.5} /> Markdown
                </Group>
              </Anchor>
            ) : null}
          </Group>
        ) : null}
      </Group>
      {a.error && !pending ? (
        <Text size="xs" c={a.available ? 'dimmed' : 'red'}>
          {a.available ? `Archiving again failed; this copy is kept. ${a.error}` : a.error}
        </Text>
      ) : null}
    </Stack>
  )
}
