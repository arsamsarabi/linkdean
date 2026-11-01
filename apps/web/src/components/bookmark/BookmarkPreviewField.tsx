import { useMemo, useState, type ReactElement } from 'react'
import { Button, FileButton, Group, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconPhoto, IconTrash } from '@tabler/icons-react'
import { AvatarCropModal } from '@/components/avatar'
import { bookmarkPreviewSrc } from '@/services'
import { ApiImage } from './ApiImage'

const iconSm = { size: 16, stroke: 1.5 } as const
/** Match server cover crop (1200×675). */
const PREVIEW_ASPECT = 16 / 9

type Props = {
  bookmarkId?: string
  hasPreview?: boolean
  previewImageUrl?: string | null
  updatedAt?: string
  /** Pending crop for a bookmark that is not saved yet. */
  pendingBlob: Blob | null
  onPendingChange: (blob: Blob | null) => void
  onUploaded: (next: { has_preview: boolean; preview_image_url?: string | null; updated_at: string }) => void
  onUpload: (
    id: string,
    blob: Blob
  ) => Promise<{
    has_preview: boolean
    preview_image_url?: string | null
    updated_at: string
  }>
  onDelete: (id: string) => Promise<{
    has_preview: boolean
    preview_image_url?: string | null
    updated_at: string
  }>
  disabled?: boolean
}

export function BookmarkPreviewField({
  bookmarkId,
  hasPreview,
  previewImageUrl,
  updatedAt,
  pendingBlob,
  onPendingChange,
  onUploaded,
  onUpload,
  onDelete,
  disabled,
}: Props): ReactElement {
  const [cropFile, setCropFile] = useState<File | null>(null)
  const [cropOpen, setCropOpen] = useState(false)
  const [busy, setBusy] = useState(false)
  const [pendingUrl, setPendingUrl] = useState<string | null>(null)

  const previewSrc = useMemo(() => {
    if (pendingUrl) return pendingUrl
    if (!bookmarkId) return undefined
    return bookmarkPreviewSrc({
      id: bookmarkId,
      has_preview: hasPreview,
      preview_image_url: previewImageUrl,
      updated_at: updatedAt,
    })
  }, [bookmarkId, hasPreview, previewImageUrl, updatedAt, pendingUrl])

  function onPick(file: File | null): void {
    if (!file) return
    setCropFile(file)
    setCropOpen(true)
  }

  async function onCropped(blob: Blob): Promise<void> {
    if (pendingUrl) URL.revokeObjectURL(pendingUrl)
    const url = URL.createObjectURL(blob)
    setPendingUrl(url)

    if (!bookmarkId) {
      onPendingChange(blob)
      notifications.show({ message: 'Image ready — will upload when you save' })
      return
    }

    setBusy(true)
    try {
      const next = await onUpload(bookmarkId, blob)
      onPendingChange(null)
      onUploaded(next)
      notifications.show({ message: 'Preview image saved' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  async function clearImage(): Promise<void> {
    if (pendingUrl) {
      URL.revokeObjectURL(pendingUrl)
      setPendingUrl(null)
    }
    onPendingChange(null)

    if (!bookmarkId || !hasPreview) {
      notifications.show({ message: 'Image cleared' })
      return
    }

    setBusy(true)
    try {
      const next = await onDelete(bookmarkId)
      onUploaded(next)
      notifications.show({ message: 'Custom preview removed' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  const showClear = !!pendingBlob || !!pendingUrl || !!hasPreview

  return (
    <>
      <Stack gap="xs">
        <Text size="sm" fw={500}>
          Preview image
        </Text>
        <Text size="xs" c="dimmed">
          Replaces the default card image. Crop to 16:9.
        </Text>
        {previewSrc ? <ApiImage src={previewSrc} alt="" radius="sm" mah={140} fit="cover" /> : null}
        <Group gap="sm">
          <FileButton onChange={onPick} accept="image/*" disabled={disabled || busy}>
            {(props) => (
              <Button {...props} variant="light" leftSection={<IconPhoto {...iconSm} />} loading={busy}>
                {previewSrc ? 'Change image' : 'Upload image'}
              </Button>
            )}
          </FileButton>
          {showClear ? (
            <Button
              variant="default"
              leftSection={<IconTrash {...iconSm} />}
              disabled={disabled || busy}
              onClick={() => void clearImage()}
            >
              Remove
            </Button>
          ) : null}
        </Group>
      </Stack>

      <AvatarCropModal
        opened={cropOpen}
        file={cropFile}
        onClose={() => {
          setCropOpen(false)
          setCropFile(null)
        }}
        onCropped={onCropped}
        aspect={PREVIEW_ASPECT}
        title="Crop preview image"
        confirmLabel="Use image"
      />
    </>
  )
}
