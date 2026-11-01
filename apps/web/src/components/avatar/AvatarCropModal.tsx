import { useCallback, useEffect, useState, type ReactElement } from 'react'
import { Button, Group, Modal, Slider, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCheck, IconX } from '@tabler/icons-react'
import Cropper, { type Area } from 'react-easy-crop'
import { useSyncDuringRender } from '@/hooks'
import { getCroppedBlob } from '@/services'
import styles from './AvatarCropModal.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  opened: boolean
  file: File | null
  onClose: () => void
  onCropped: (blob: Blob) => Promise<void> | void
  /** Crop aspect ratio. Default 1 (square avatar). */
  aspect?: number
  title?: string
  confirmLabel?: string
}

export function AvatarCropModal({
  opened,
  file,
  onClose,
  onCropped,
  aspect = 1,
  title = 'Crop avatar',
  confirmLabel = 'Save avatar',
}: Props): ReactElement {
  const [crop, setCrop] = useState({ x: 0, y: 0 })
  const [zoom, setZoom] = useState(1)
  const [area, setArea] = useState<Area | null>(null)
  const [busy, setBusy] = useState(false)
  const [src, setSrc] = useState<string | null>(null)

  useSyncDuringRender([file], () => {
    setSrc(null)
    setCrop({ x: 0, y: 0 })
    setZoom(1)
    setArea(null)
  })

  useEffect(() => {
    if (!file) return
    const url = URL.createObjectURL(file)
    // oxlint-disable-next-line react/set-state-in-effect -- the object URL is created and revoked in this effect
    setSrc(url)
    return () => {
      URL.revokeObjectURL(url)
    }
  }, [file])

  const onCropComplete = useCallback((_a: Area, pixels: Area): void => {
    setArea(pixels)
  }, [])

  async function save(): Promise<void> {
    if (!src || !area) return
    setBusy(true)
    try {
      const blob = await getCroppedBlob(src, area)
      await onCropped(blob)
      onClose()
    } catch (e) {
      notifications.show({
        color: 'red',
        message: e instanceof Error ? e.message : String(e),
      })
    } finally {
      setBusy(false)
    }
  }

  return (
    <Modal opened={opened} onClose={onClose} title={title} size="lg">
      <Stack>
        {src ? (
          <div className={styles.cropArea}>
            <Cropper
              image={src}
              crop={crop}
              zoom={zoom}
              aspect={aspect}
              onCropChange={setCrop}
              onZoomChange={setZoom}
              onCropComplete={onCropComplete}
            />
          </div>
        ) : (
          <Text c="dimmed">No image selected</Text>
        )}
        <Text size="sm">Zoom</Text>
        <Slider min={1} max={3} step={0.05} value={zoom} onChange={setZoom} />
        <Group justify="flex-end">
          <Button variant="default" leftSection={<IconX {...iconSm} />} onClick={onClose}>
            Cancel
          </Button>
          <Button leftSection={<IconCheck {...iconSm} />} loading={busy} onClick={() => void save()} disabled={!area}>
            {confirmLabel}
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
