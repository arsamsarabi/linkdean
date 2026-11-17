import { useState, type ReactElement } from 'react'
import { Alert, Button, Checkbox, Group, Modal, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useBulkArchive, useMe, useRequestArchive } from '@/hooks'
import { useChildDialog } from '@/hooks/render'
import { formatArchiveDate } from '@/lib'
import type { ArchiveFormat } from '@/schemas'

type Props = {
  opened: boolean
  onClose: () => void
  /** One bookmark, or the bulk bar's selection. */
  ids: string[]
  /** When the copy this replaces was taken (one bookmark that already has a copy). */
  replaces?: string | null
}

type PageFormat = Extract<ArchiveFormat, 'html' | 'pdf'>

/** Asks which formats to keep, then queues the archive. */
export function ArchiveDialog({ opened, onClose, ids, replaces }: Props): ReactElement {
  useChildDialog(opened)
  return (
    <Modal
      opened={opened}
      onClose={onClose}
      title={ids.length > 1 ? `Archive ${ids.length} bookmarks` : 'Archive'}
      centered
    >
      {opened ? <ArchiveForm ids={ids} replaces={replaces} onClose={onClose} /> : null}
    </Modal>
  )
}

function ArchiveForm({ ids, replaces, onClose }: Omit<Props, 'opened'>): ReactElement {
  const defaults = useMe().data?.settings.archive_formats ?? ['html']
  const [formats, setFormats] = useState<PageFormat[]>(defaults)
  const one = useRequestArchive()
  const bulk = useBulkArchive()
  const pending = one.isPending || bulk.isPending

  async function submit(): Promise<void> {
    try {
      if (ids.length === 1) {
        await one.mutateAsync({ id: ids[0], formats })
        notifications.show({ message: 'Archiving. You can keep working; the copy appears when it is ready.' })
      } else {
        const res = await bulk.mutateAsync({ ids, formats })
        notifications.show({
          color: res.failed.length ? 'yellow' : undefined,
          message: res.failed.length
            ? `Archiving ${res.ok} of ${ids.length}. ${res.failed[0].error}`
            : `Archiving ${res.ok} bookmarks. Copies appear as they are ready.`,
        })
      }
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Stack gap="md">
      <Checkbox.Group label="Keep the page as" value={formats} onChange={(v) => setFormats(v as PageFormat[])}>
        <Stack gap="xs" mt="xs">
          <Checkbox
            value="html"
            label="Web page (HTML)"
            description="Text, images and styles in one file you can read here."
          />
          <Checkbox value="pdf" label="PDF" description="A printable snapshot." />
        </Stack>
      </Checkbox.Group>
      <Text size="xs" c="dimmed">
        An address that serves a file, such as a PDF or an image, is kept as that file.
      </Text>
      {replaces ? (
        <Alert color="yellow" variant="light">
          This replaces the copy from {formatArchiveDate(replaces)}. If archiving fails, the current copy is kept.
        </Alert>
      ) : null}
      <Group justify="flex-end" gap="sm">
        <Button variant="default" onClick={onClose} disabled={pending}>
          Cancel
        </Button>
        <Button onClick={() => void submit()} loading={pending} disabled={formats.length === 0}>
          Archive
        </Button>
      </Group>
    </Stack>
  )
}
