import { useState, type ReactElement } from 'react'
import {
  Alert,
  Badge,
  Button,
  Drawer,
  Group,
  Pagination,
  Paper,
  SegmentedControl,
  SimpleGrid,
  Stack,
  Text,
} from '@mantine/core'
import { useDrawerSize } from '@/hooks/render'
import type { DuplicateAction, DuplicateDecisions, ImportDuplicate, ImportPreview } from '@/schemas'
import { actionFor, decisionsBody } from './duplicateDecisions'

const PAGE = 50

const choices: { value: DuplicateAction; label: string }[] = [
  { value: 'skip', label: 'Skip' },
  { value: 'replace', label: 'Replace' },
  { value: 'keep', label: 'Import anyway' },
]

type Props = {
  opened: boolean
  preview: ImportPreview
  pending: boolean
  error: string | null
  onConfirm: (decisions: DuplicateDecisions) => void
  onClose: () => void
}

function Side({
  label,
  title,
  description,
  tags,
  folders,
}: { label: string } & Pick<ImportDuplicate['incoming'], 'title' | 'description' | 'tags' | 'folders'>): ReactElement {
  return (
    <Stack gap={2}>
      <Text size="xs" c="dimmed">
        {label}
      </Text>
      <Text size="sm" fw={500}>
        {title || 'No title'}
      </Text>
      {description ? (
        <Text size="xs" lineClamp={2}>
          {description}
        </Text>
      ) : null}
      <Text size="xs" c="dimmed">
        {folders.length ? folders.join(', ') : 'No folder'}
      </Text>
      {tags.length ? (
        <Group gap={4}>
          {tags.map((t) => (
            <Badge key={t} size="xs" variant="light">
              {t}
            </Badge>
          ))}
        </Group>
      ) : null}
    </Stack>
  )
}

export function ImportDuplicatesDrawer({ opened, preview, pending, error, onConfirm, onClose }: Props): ReactElement {
  const drawerSize = useDrawerSize()
  const [all, setAll] = useState<DuplicateAction>('skip')
  const [picked, setPicked] = useState<Map<number, DuplicateAction>>(new Map())
  const [page, setPage] = useState(1)
  const rows = preview.duplicates.slice((page - 1) * PAGE, page * PAGE)
  const pages = Math.ceil(preview.duplicates.length / PAGE)

  return (
    <Drawer opened={opened} onClose={onClose} title="Review duplicates" position="right" size={drawerSize} padding="lg">
      <Stack gap="md">
        <Text>{`${preview.duplicates.length} of ${preview.total} bookmarks are already in your library.`}</Text>
        <Stack gap={4} role="group" aria-label="Apply to all">
          <Text size="sm" fw={500}>
            Apply to all
          </Text>
          <SegmentedControl data={choices} value={all} onChange={(v) => setAll(v as DuplicateAction)} fullWidth />
        </Stack>
        {rows.map((d) => (
          <Paper key={d.index} withBorder p="sm" radius="md" role="group" aria-label={d.url}>
            <Stack gap="xs">
              <Text size="xs" truncate="end">
                {d.url}
              </Text>
              <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="sm">
                <Side label="In your library" {...d.existing} />
                <Side label="In the file" {...d.incoming} />
              </SimpleGrid>
              <SegmentedControl
                size="xs"
                data={choices}
                value={actionFor(d.index, all, picked)}
                onChange={(v) => setPicked((m) => new Map(m).set(d.index, v as DuplicateAction))}
              />
            </Stack>
          </Paper>
        ))}
        {pages > 1 ? <Pagination value={page} onChange={setPage} total={pages} /> : null}
        {error ? <Alert color="red">{error}</Alert> : null}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose} disabled={pending}>
            Cancel
          </Button>
          <Button onClick={() => onConfirm(decisionsBody(all, picked))} loading={pending}>
            Import
          </Button>
        </Group>
      </Stack>
    </Drawer>
  )
}
