import { type ReactElement } from 'react'
import { Button, Center, Group, SegmentedControl, Stack, Text, Title, Tooltip } from '@mantine/core'
import {
  IconBookmarkPlus,
  IconCheck,
  IconCopy,
  IconFolders,
  IconLayoutGrid,
} from '@tabler/icons-react'
import type { LibraryViewMode } from '@/hooks'
import styles from './LibraryPage.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  loading: boolean
  viewMode: LibraryViewMode
  listCount: number
  searching: boolean
  selectMode: boolean
  canCreate: boolean
  onViewModeChange: (mode: LibraryViewMode) => void
  onEnterSelect: () => void
  onExitSelect: () => void
  onOpenDupes: () => void
  onCreate: () => void
}

export function LibraryPageHeader({
  loading,
  viewMode,
  listCount,
  searching,
  selectMode,
  canCreate,
  onViewModeChange,
  onEnterSelect,
  onExitSelect,
  onOpenDupes,
  onCreate,
}: Props): ReactElement {
  return (
    <Group justify="space-between" align="flex-end" wrap="wrap" gap="md">
      <Stack gap={4}>
        <Title order={2}>Library</Title>
        <Text size="sm" c="dimmed" aria-live="polite" className={styles.countLine}>
          {!loading && viewMode === 'grid'
            ? `${listCount} bookmark${listCount === 1 ? '' : 's'}${searching ? ' matched' : ''}`
            : '\u00a0'}
        </Text>
      </Stack>
      <Group gap="sm" wrap="wrap">
        <SegmentedControl
          aria-label="Library view"
          value={viewMode}
          onChange={(v) => onViewModeChange(v as LibraryViewMode)}
          data={[
            {
              value: 'grid',
              label: (
                <Center style={{ gap: 6 }}>
                  <IconLayoutGrid size={16} stroke={1.5} />
                  <span>Grid</span>
                </Center>
              ),
            },
            {
              value: 'explorer',
              label: (
                <Center style={{ gap: 6 }}>
                  <IconFolders size={16} stroke={1.5} />
                  <span>Explorer</span>
                </Center>
              ),
            },
          ]}
        />
        {selectMode ? (
          <Button variant="default" onClick={onExitSelect}>
            Cancel select
          </Button>
        ) : (
          <>
            <Button
              variant="default"
              leftSection={<IconCheck {...iconSm} />}
              onClick={onEnterSelect}
              disabled={loading || listCount === 0}
            >
              Select
            </Button>
            <Tooltip label="Find and merge duplicate URLs">
              <Button variant="default" leftSection={<IconCopy {...iconSm} />} onClick={onOpenDupes}>
                Duplicates
              </Button>
            </Tooltip>
          </>
        )}
        {canCreate ? (
          <Tooltip label="New bookmark (N)">
            <Button leftSection={<IconBookmarkPlus {...iconSm} />} onClick={onCreate}>
              Add bookmark
            </Button>
          </Tooltip>
        ) : (
          <Tooltip label="Read-only shared folder">
            <Button leftSection={<IconBookmarkPlus {...iconSm} />} disabled>
              Add bookmark
            </Button>
          </Tooltip>
        )}
      </Group>
    </Group>
  )
}
