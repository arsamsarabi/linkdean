import { useState, type ReactElement } from 'react'
import { Alert, Button, FileInput, Group, Paper, Select, SimpleGrid, Stack, ThemeIcon, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconDownload, IconFileExport, IconInfoCircle, IconUpload } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { useExportBookmarks, useImportBookmarks } from '@/hooks'

const iconSm = { size: 16, stroke: 1.5 } as const

export function ImportExportPanel(): ReactElement {
  const [format, setFormat] = useState<string | null>('netscape')
  const [file, setFile] = useState<File | null>(null)
  const importMut = useImportBookmarks()
  const { exportBookmarks, isExporting, exportError, clearExportError } = useExportBookmarks()

  async function doImport(): Promise<void> {
    if (!file || !format) return
    try {
      const content = await file.text()
      const res = await importMut.mutateAsync({ format, content })
      notifications.show({
        message: `Imported ${res.imported}/${res.total} (failed ${res.failed})`,
      })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function exportFile(fmt: string): Promise<void> {
    try {
      await exportBookmarks(fmt)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <SimpleGrid cols={{ base: 1, md: 2 }} spacing="lg">
      <Paper p="xl" withBorder>
        <Stack gap="lg">
          <Group gap="sm">
            <ThemeIcon size={28} radius="md" variant="light" color="violet">
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
            loading={importMut.isPending}
            disabled={!file}
            onClick={() => void doImport()}
          >
            Import
          </Button>
        </Stack>
      </Paper>

      <Paper p="xl" withBorder>
        <Stack gap="lg">
          <Group gap="sm">
            <ThemeIcon size={28} radius="md" variant="light" color="violet">
              <IconFileExport size={16} stroke={1.5} />
            </ThemeIcon>
            <Title order={4}>Export</Title>
          </Group>
          <Alert icon={<IconInfoCircle size={16} />} color="violet" variant="light">
            Vault bookmarks skipped on export — ciphertext stays private.
          </Alert>
          {exportError ? (
            <QueryErrorAlert
              error={exportError}
              onRetry={() => {
                clearExportError()
              }}
              title="Export failed"
            />
          ) : null}
          <Group>
            <Button
              variant="light"
              leftSection={<IconFileExport {...iconSm} />}
              loading={isExporting}
              onClick={() => void exportFile('json')}
            >
              JSON
            </Button>
            <Button
              variant="light"
              leftSection={<IconFileExport {...iconSm} />}
              loading={isExporting}
              onClick={() => void exportFile('html')}
            >
              Netscape HTML
            </Button>
          </Group>
        </Stack>
      </Paper>
    </SimpleGrid>
  )
}
