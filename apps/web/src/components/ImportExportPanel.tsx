import { useState } from 'react'
import {
  Alert,
  Button,
  FileInput,
  Group,
  Paper,
  Select,
  Stack,
  ThemeIcon,
  Title,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  IconDownload,
  IconFileExport,
  IconInfoCircle,
  IconUpload,
} from '@tabler/icons-react'
import { api, getToken } from '../api'

const iconSm = { size: 16, stroke: 1.5 } as const

export function ImportExportPanel() {
  const [format, setFormat] = useState<string | null>('netscape')
  const [file, setFile] = useState<File | null>(null)
  const [busy, setBusy] = useState(false)

  async function doImport() {
    if (!file || !format) return
    setBusy(true)
    try {
      const content = await file.text()
      const res = await api.import(format, content)
      notifications.show({
        message: `Imported ${res.imported}/${res.total} (failed ${res.failed})`,
      })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setBusy(false)
    }
  }

  function exportFile(fmt: string) {
    const token = getToken()
    const a = document.createElement('a')
    a.href = api.exportUrl(fmt)
    if (token) {
      fetch(api.exportUrl(fmt), { headers: { Authorization: `Bearer ${token}` } })
        .then((r) => r.blob())
        .then((blob) => {
          a.href = URL.createObjectURL(blob)
          a.download = fmt === 'html' ? 'linkdean-bookmarks.html' : 'linkdean-export.json'
          a.click()
        })
    }
  }

  return (
    <Stack gap="lg" maw={720}>
      <Paper p="xl" withBorder>
        <Stack gap="lg">
          <Group gap="sm">
            <ThemeIcon size={28} radius="md" variant="light" color="harbor">
              <IconUpload size={16} stroke={1.5} />
            </ThemeIcon>
            <Title order={4}>Import</Title>
          </Group>
          <Select
            label="Format"
            value={format}
            onChange={setFormat}
            data={[
              { value: 'netscape', label: 'Netscape HTML (Chrome/Firefox)' },
              { value: 'json', label: 'Linkdean JSON' },
              { value: 'pocket', label: 'Pocket CSV' },
              { value: 'raindrop', label: 'Raindrop CSV' },
            ]}
          />
          <FileInput
            label="File"
            value={file}
            onChange={setFile}
            clearable
            leftSection={<IconDownload {...iconSm} />}
          />
          <Button
            leftSection={<IconUpload {...iconSm} />}
            loading={busy}
            disabled={!file}
            onClick={doImport}
          >
            Import
          </Button>
        </Stack>
      </Paper>

      <Paper p="xl" withBorder>
        <Stack gap="lg">
          <Group gap="sm">
            <ThemeIcon size={28} radius="md" variant="light" color="harbor">
              <IconFileExport size={16} stroke={1.5} />
            </ThemeIcon>
            <Title order={4}>Export</Title>
          </Group>
          <Alert icon={<IconInfoCircle size={16} />} color="harbor" variant="light">
            Vault bookmarks skipped on export — ciphertext stays private.
          </Alert>
          <Group>
            <Button
              variant="light"
              leftSection={<IconFileExport {...iconSm} />}
              onClick={() => exportFile('json')}
            >
              JSON
            </Button>
            <Button
              variant="light"
              leftSection={<IconFileExport {...iconSm} />}
              onClick={() => exportFile('html')}
            >
              Netscape HTML
            </Button>
          </Group>
        </Stack>
      </Paper>
    </Stack>
  )
}
