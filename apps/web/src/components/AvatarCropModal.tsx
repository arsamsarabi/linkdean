import { useCallback, useState } from 'react'
import Cropper, { type Area } from 'react-easy-crop'
import { Button, Group, Modal, Slider, Stack, Text } from '@mantine/core'
import { IconCheck, IconX } from '@tabler/icons-react'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  opened: boolean
  file: File | null
  onClose: () => void
  onCropped: (blob: Blob) => Promise<void> | void
}

async function getCroppedBlob(imageSrc: string, crop: Area): Promise<Blob> {
  const image = await createImage(imageSrc)
  const canvas = document.createElement('canvas')
  const size = Math.max(crop.width, crop.height)
  canvas.width = size
  canvas.height = size
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('canvas unavailable')
  ctx.drawImage(
    image,
    crop.x,
    crop.y,
    crop.width,
    crop.height,
    0,
    0,
    size,
    size,
  )
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (b) => (b ? resolve(b) : reject(new Error('encode failed'))),
      'image/jpeg',
      0.95,
    )
  })
}

function createImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image()
    img.addEventListener('load', () => resolve(img))
    img.addEventListener('error', (e) => reject(e))
    img.src = url
  })
}

export function AvatarCropModal({ opened, file, onClose, onCropped }: Props) {
  const [crop, setCrop] = useState({ x: 0, y: 0 })
  const [zoom, setZoom] = useState(1)
  const [area, setArea] = useState<Area | null>(null)
  const [busy, setBusy] = useState(false)
  const src = file ? URL.createObjectURL(file) : null

  const onCropComplete = useCallback((_a: Area, pixels: Area) => {
    setArea(pixels)
  }, [])

  async function save() {
    if (!src || !area) return
    setBusy(true)
    try {
      const blob = await getCroppedBlob(src, area)
      await onCropped(blob)
      onClose()
    } finally {
      setBusy(false)
    }
  }

  return (
    <Modal opened={opened} onClose={onClose} title="Crop avatar" size="lg">
      <Stack>
        {src ? (
          <div style={{ position: 'relative', width: '100%', height: 320, background: '#111' }}>
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
            onClick={save}
            disabled={!area}
          >
            Save avatar
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
