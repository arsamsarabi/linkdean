import { type ReactElement } from 'react'
import { Anchor, Box, Button, Group, Stack, Text, Title, Tooltip } from '@mantine/core'
import { useMediaQuery } from '@mantine/hooks'
import {
  IconBookmark,
  IconBookmarkPlus,
  IconCheck,
  IconFolders,
  IconLayoutGrid,
  IconList,
  IconPin,
  IconStar,
} from '@tabler/icons-react'
import type { LibraryViewMode } from '@/hooks'
import { FOLDER_ALL, type FlagFilter } from '@/services'
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
  { mode: 'list', label: 'List', Icon: IconList },
  { mode: 'explorer', label: 'Explorer', Icon: IconFolders },
]

type ToolbarButtonProps = {
  label: string
  Icon: typeof IconStar
  compact: boolean
  active?: boolean
  pressed?: boolean
  disabled?: boolean
  onClick: () => void
}

/** Labelled on wide screens; icon-only with a tooltip on phones, so the toolbar fits one row. */
function ToolbarButton({ label, Icon, compact, active, pressed, disabled, onClick }: ToolbarButtonProps): ReactElement {
  return (
    <Tooltip label={label} disabled={!compact}>
      <Button
        variant={active ? 'light' : 'default'}
        leftSection={<Icon {...iconSm} />}
        aria-label={label}
        aria-pressed={pressed}
        disabled={disabled}
        onClick={onClick}
        px={compact ? 'xs' : undefined}
        styles={compact ? { section: { marginInlineEnd: 0 } } : undefined}
      >
        <Box component="span" visibleFrom="sm">
          {label}
        </Box>
      </Button>
    </Tooltip>
  )
}

export function LibraryPageHeader(): ReactElement {
  const compact = useMediaQuery('(max-width: 48em)') ?? false
  const lib = useLibraryPageModel()
  const { loading, viewMode, searching, selectMode, canCreateInContext: canCreate } = lib
  const listCount = lib.list.length

  return (
    <Group justify="space-between" align="flex-end" wrap="wrap" gap="md">
      <Stack gap={4}>
        {lib.folderCat ? (
          <Text size="sm" c="dimmed">
            <Anchor
              component="button"
              type="button"
              size="sm"
              c="dimmed"
              onClick={() => lib.setLibrarySearch({ folder: FOLDER_ALL })}
            >
              {lib.scope === 'vault' ? 'Vaults' : 'Library'}
            </Anchor>{' '}
            /
          </Text>
        ) : null}
        <Title order={2}>{lib.folderCat?.name ?? (lib.scope === 'vault' ? 'Vaults' : 'Library')}</Title>
        <Text size="sm" c="dimmed" aria-live="polite" className={styles.countLine}>
          {!loading && viewMode !== 'explorer'
            ? `${listCount} bookmark${listCount === 1 ? '' : 's'}${searching ? ' matched' : ''}`
            : '\u00a0'}
        </Text>
      </Stack>
      <Group gap="sm" wrap="wrap">
        <Button.Group aria-label="Quick filters">
          {FLAG_FILTERS.map(({ flag, label, Icon }) => {
            const active = lib.filterFlags.includes(flag)
            return (
              <ToolbarButton
                key={flag}
                label={label}
                Icon={Icon}
                compact={compact}
                active={active}
                pressed={active}
                onClick={() => lib.toggleFlagFilter(flag)}
              />
            )
          })}
        </Button.Group>
        <Button.Group aria-label="Library view">
          {VIEW_MODES.map(({ mode, label, Icon }) => (
            <ToolbarButton
              key={mode}
              label={label}
              Icon={Icon}
              compact={compact}
              active={viewMode === mode}
              pressed={viewMode === mode}
              onClick={() => lib.onViewModeChange(mode)}
            />
          ))}
        </Button.Group>
        {selectMode ? (
          <Button variant="default" onClick={lib.exitSelectMode}>
            Cancel select
          </Button>
        ) : (
          <ToolbarButton
            label="Select"
            Icon={IconCheck}
            compact={compact}
            onClick={lib.enterSelectMode}
            disabled={loading || listCount === 0}
          />
        )}
        {/* On phones the tab bar's Add adds bookmarks, into the open folder. */}
        <Box visibleFrom="sm">
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
        </Box>
      </Group>
    </Group>
  )
}
