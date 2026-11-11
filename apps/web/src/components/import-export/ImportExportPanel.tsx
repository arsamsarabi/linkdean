import { useState, type ReactElement } from 'react'
import { Alert, Button, FileInput, Group, Paper, Select, SimpleGrid, Stack, ThemeIcon, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconDownload, IconFileExport, IconInfoCircle, IconUpload } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { useExportBookmarks, useImportBookmarks, useImportPreview, useMe } from '@/hooks'
import type { DuplicateDecisions, ImportPreview, ImportResult } from '@/schemas'
import { ImportDuplicatesDrawer } from './ImportDuplicatesDrawer'

const iconSm = { size: 16, stroke: 1.5 } as const

function resultMessage(res: ImportResult): string {
  const parts = [`Imported ${res.imported} of ${res.total}`]
  if (res.replaced) parts.push(`replaced ${res.replaced}`)
  if (res.skipped) parts.push(`skipped ${res.skipped}`)
  if (res.failed) parts.push(`failed ${res.failed}`)
  return parts.join(', ')
}

export function ImportExportPanel(): ReactElement {
  const [format, setFormat] = useState<string | null>('netscape')
  const [file, setFile] = useState<File | null>(null)
  const importMut = useImportBookmarks()
  const vaultUnlocked = !!useMe().data?.vault_unlocked
  const { exportBookmarks, isExporting, exportError, clearExportError } = useExportBookmarks()

  const previewMut = useImportPreview()
  // A file with duplicates, waiting for the person's choices in the review drawer.
  const [review, setReview] = useState<{ format: string; content: string; preview: ImportPreview } | null>(null)
  const [reviewError, setReviewError] = useState<string | null>(null)

  async function runImport(fmt: string, content: string, duplicates?: DuplicateDecisions): Promise<void> {
    const res = await importMut.mutateAsync(
      duplicates ? { format: fmt, content, duplicates } : { format: fmt, content }
    )
    notifications.show({ message: resultMessage(res) })
  }

  async function doImport(): Promise<void> {
    if (!file || !format) return
    try {
      const content = await file.text()
      const preview = await previewMut.mutateAsync({ format, content })
      if (preview.duplicates.length === 0) {
        await runImport(format, content)
        return
      }
      setReviewError(null)
      setReview({ format, content, preview })
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  async function confirmReview(decisions: DuplicateDecisions): Promise<void> {
    if (!review) return
    try {
      await runImport(review.format, review.content, decisions)
      setReview(null)
    } catch (e) {
      // The drawer stays open with the error, so the person can retry without choosing again.
      setReviewError(e instanceof Error ? e.message : String(e))
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
    <>
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
              loading={importMut.isPending || previewMut.isPending}
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
              {vaultUnlocked
                ? 'Vault is unlocked, so vault items are included in the export as plain text. Keep the file safe.'
                : 'Vault items are skipped while the vault is locked. Unlock it first to include them.'}
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
      {review ? (
        <ImportDuplicatesDrawer
          opened
          preview={review.preview}
          pending={importMut.isPending}
          error={reviewError}
          onConfirm={(d) => void confirmReview(d)}
          onClose={() => setReview(null)}
        />
      ) : null}
    </>
  )
}
