import { useCallback, useEffect, useState, type ReactElement } from 'react'
import Cropper, { type Area } from 'react-easy-crop'
import { Button, Group, Modal, Slider, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCheck, IconX } from '@tabler/icons-react'
import { getCroppedBlob } from '../../services/avatar/avatarCrop'
import styles from './AvatarCropModal.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  opened: boolean
  file: File | null
  onClose: () => void
  onCropped: (blob: Blob) => Promise<void> | void
}

export function AvatarCropModal({ opened, file, onClose, onCropped }: Props): ReactElement {
  const [crop, setCrop] = useState({ x: 0, y: 0 })
  const [zoom, setZoom] = useState(1)
  const [area, setArea] = useState<Area | null>(null)
  const [busy, setBusy] = useState(false)
  const [src, setSrc] = useState<string | null>(null)

  useEffect(() => {
    if (!file) {
      setSrc(null)
      return
    }
    const url = URL.createObjectURL(file)
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
    <Modal opened={opened} onClose={onClose} title="Crop avatar" size="lg">
      <Stack>
        {src ? (
          <div className={styles.cropArea}>
            <Cropper
              image={src}
              crop={crop}
              zoom={zoom}
              aspect={1}
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
          <Button
            leftSection={<IconCheck {...iconSm} />}
            loading={busy}
            onClick={() => void save()}
            disabled={!area}
          >
            Save avatar
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
