import { useState, type ReactElement } from 'react'
import { ActionIcon, Group, NavLink, Skeleton, Stack, Text, UnstyledButton } from '@mantine/core'
import { useLocalStorage } from '@mantine/hooks'
import { notifications } from '@mantine/notifications'
import { IconChevronDown, IconDownload, IconPencil, IconSearch, IconTrash } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { ConfirmModal } from '@/components'
import {
  useDeleteSavedSearch,
  useDownloadAgentSkill,
  useExperimentalFlags,
  usePatchSavedSearch,
  useSavedSearches,
} from '@/hooks'
import { librarySearchFromSavedFilters, pathForSavedFilters } from '@/lib'
import type { SavedSearch } from '@/schemas'
import { SaveSearchModal } from '../library/SaveSearchModal'
import classes from './AppShell.module.css'
import { EXACT } from './sidebarLinks'

const iconProps = { size: 18, stroke: 1.5 } as const

type Props = {
  onNavigate: () => void
}

/**
 * Collapsible list of named library filter snapshots. Click opens the library (or vaults)
 * with that search applied.
 */
export function SidebarSavedSearches({ onNavigate }: Props): ReactElement {
  const list = useSavedSearches()
  const patch = usePatchSavedSearch()
  const remove = useDeleteSavedSearch()
  const experimental = useExperimentalFlags()
  const agentSkill = useDownloadAgentSkill()
  const [expanded, setExpanded] = useLocalStorage({
    key: 'linkdean-sidebar-saved-searches-expanded',
    defaultValue: true,
    getInitialValueInEffect: false,
  })
  const [renameTarget, setRenameTarget] = useState<SavedSearch | null>(null)
  const [deleteId, setDeleteId] = useState<string | null>(null)

  const items = list.data ?? []

  return (
    <Stack gap={2}>
      <UnstyledButton
        className={classes.groupToggle}
        aria-expanded={expanded}
        aria-controls="sidebar-saved-searches"
        onClick={() => setExpanded((v) => !v)}
      >
        <Group justify="space-between" wrap="nowrap" px="sm" pt="sm" pb={4}>
          <Text size="xs" fw={600} c="dimmed" tt="uppercase" className={classes.groupLabel}>
            Saved searches
          </Text>
          <IconChevronDown
            size={14}
            stroke={1.5}
            className={classes.groupChevron}
            data-expanded={expanded || undefined}
            aria-hidden
          />
        </Group>
      </UnstyledButton>

      {expanded ? (
        <Stack gap={2} id="sidebar-saved-searches">
          {list.isLoading ? (
            <Stack gap={6} px="sm">
              <Skeleton height={20} />
              <Skeleton height={20} />
            </Stack>
          ) : items.length === 0 ? (
            <Text size="sm" c="dimmed" px="sm" py={6}>
              Save a search from the library.
            </Text>
          ) : (
            items.map((s) => (
              <NavLink
                key={s.id}
                renderRoot={(props) => (
                  <Link
                    {...props}
                    to={pathForSavedFilters(s.filters)}
                    search={librarySearchFromSavedFilters(s.filters)}
                    activeOptions={EXACT}
                  />
                )}
                label={s.name}
                leftSection={<IconSearch {...iconProps} />}
                onClick={onNavigate}
                rightSection={
                  <Group gap={2} wrap="nowrap" onClick={(e) => e.preventDefault()}>
                    {experimental.agent_skills ? (
                      <ActionIcon
                        size="sm"
                        variant="subtle"
                        aria-label={`Download Agent Skill for ${s.name}`}
                        loading={agentSkill.pending}
                        onClick={(e) => {
                          e.preventDefault()
                          e.stopPropagation()
                          void agentSkill.download({ search: s.id })
                        }}
                      >
                        <IconDownload size={14} stroke={1.5} />
                      </ActionIcon>
                    ) : null}
                    <ActionIcon
                      size="sm"
                      variant="subtle"
                      aria-label={`Rename ${s.name}`}
                      onClick={(e) => {
                        e.preventDefault()
                        e.stopPropagation()
                        setRenameTarget(s)
                      }}
                    >
                      <IconPencil size={14} stroke={1.5} />
                    </ActionIcon>
                    <ActionIcon
                      size="sm"
                      variant="subtle"
                      color="red"
                      aria-label={`Delete ${s.name}`}
                      onClick={(e) => {
                        e.preventDefault()
                        e.stopPropagation()
                        setDeleteId(s.id)
                      }}
                    >
                      <IconTrash size={14} stroke={1.5} />
                    </ActionIcon>
                  </Group>
                }
              />
            ))
          )}
        </Stack>
      ) : null}

      <SaveSearchModal
        opened={!!renameTarget}
        onClose={() => setRenameTarget(null)}
        title="Rename saved search"
        initialName={renameTarget?.name ?? ''}
        confirmLabel="Rename"
        loading={patch.isPending}
        onSubmit={async (name) => {
          if (!renameTarget) return
          try {
            await patch.mutateAsync({ id: renameTarget.id, name })
            setRenameTarget(null)
            notifications.show({ message: 'Renamed' })
          } catch (e) {
            notifications.show({ color: 'red', message: String(e) })
          }
        }}
      />

      <ConfirmModal
        opened={!!deleteId}
        onClose={() => setDeleteId(null)}
        title="Delete saved search?"
        body="This only removes the saved filter. Your bookmarks stay."
        confirmLabel="Delete"
        confirmColor="red"
        loading={remove.isPending}
        onConfirm={() => {
          if (!deleteId) return
          void remove
            .mutateAsync(deleteId)
            .then(() => {
              setDeleteId(null)
              notifications.show({ message: 'Deleted' })
            })
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
      />
    </Stack>
  )
}
