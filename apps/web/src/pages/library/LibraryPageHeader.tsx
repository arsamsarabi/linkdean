import { type ReactElement } from 'react'
import { Button, Group, Stack, Text, Title, Tooltip } from '@mantine/core'
import {
  IconBookmark,
  IconBookmarkPlus,
  IconCheck,
  IconFolders,
  IconLayoutGrid,
  IconPin,
  IconStar,
} from '@tabler/icons-react'
import type { LibraryViewMode } from '@/hooks'
import type { FlagFilter } from '@/services'
import styles from './LibraryPage.module.css'
import { useLibraryPageModel } from './useLibraryPageModel'

const iconSm = { size: 16, stroke: 1.5 } as const

const FLAG_FILTERS: { flag: FlagFilter; label: string; Icon: typeof IconStar }[] = [
  { flag: 'unread', label: 'Reading list', Icon: IconBookmark },
  { flag: 'favorite', label: 'Favorites', Icon: IconStar },
  { flag: 'pinned', label: 'Pinned', Icon: IconPin },
]

const VIEW_MODES: { mode: LibraryViewMode; label: string; Icon: typeof IconStar }[] = [
  { mode: 'grid', label: 'Grid', Icon: IconLayoutGrid },
  { mode: 'explorer', label: 'Explorer', Icon: IconFolders },
]

export function LibraryPageHeader(): ReactElement {
  const lib = useLibraryPageModel()
  const { loading, viewMode, searching, selectMode, canCreateInContext: canCreate } = lib
  const listCount = lib.list.length

  return (
    <Group justify="space-between" align="flex-end" wrap="wrap" gap="md">
      <Stack gap={4}>
        <Title order={2}>{lib.scope === 'vault' ? 'Vaults' : 'Library'}</Title>
        <Text size="sm" c="dimmed" aria-live="polite" className={styles.countLine}>
          {!loading && viewMode === 'grid'
            ? `${listCount} bookmark${listCount === 1 ? '' : 's'}${searching ? ' matched' : ''}`
            : '\u00a0'}
        </Text>
      </Stack>
      <Group gap="sm" wrap="wrap">
        <Button.Group aria-label="Quick filters">
          {FLAG_FILTERS.map(({ flag, label, Icon }) => {
            const active = lib.filterFlags.includes(flag)
            return (
              <Button
                key={flag}
                variant={active ? 'light' : 'default'}
                leftSection={<Icon {...iconSm} />}
                aria-pressed={active}
                onClick={() => lib.toggleFlagFilter(flag)}
              >
                {label}
              </Button>
            )
          })}
        </Button.Group>
        <Button.Group aria-label="Library view">
          {VIEW_MODES.map(({ mode, label, Icon }) => (
            <Button
              key={mode}
              variant={viewMode === mode ? 'light' : 'default'}
              leftSection={<Icon {...iconSm} />}
              aria-pressed={viewMode === mode}
              onClick={() => lib.onViewModeChange(mode)}
            >
              {label}
            </Button>
          ))}
        </Button.Group>
        {selectMode ? (
          <Button variant="default" onClick={lib.exitSelectMode}>
            Cancel select
          </Button>
        ) : (
          <Button
            variant="default"
            leftSection={<IconCheck {...iconSm} />}
            onClick={lib.enterSelectMode}
            disabled={loading || listCount === 0}
          >
            Select
          </Button>
        )}
        {canCreate ? (
          <Tooltip label="New bookmark (N)">
            <Button leftSection={<IconBookmarkPlus {...iconSm} />} onClick={lib.openCreate}>
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
