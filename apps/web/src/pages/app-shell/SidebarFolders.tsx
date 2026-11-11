import { useState, type ReactElement } from 'react'
import { Badge, CloseButton, Group, NavLink, Skeleton, Stack, Text, TextInput, UnstyledButton } from '@mantine/core'
import { useLocalStorage } from '@mantine/hooks'
import { IconChevronDown, IconFolder, IconInbox, IconSearch, IconUsers } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { useCategories, useUnsortedCount } from '@/hooks'
import { buildCategoryTree, CategoryIcon, filterFolders, isCategoryIconKey, type CatNode } from '@/lib'
import type { Category } from '@/schemas'
import { FOLDER_UNCATEGORIZED } from '@/services'
import classes from './AppShell.module.css'
import { recentFolders } from './folderLists'
import { EXACT } from './sidebarLinks'

const iconProps = { size: 18, stroke: 1.5 } as const

type Props = {
  /** The open library folder, if the library is showing one. */
  activeFolder: string | undefined
  onNavigate: () => void
}

function containsFolder(node: CatNode, id: string | undefined): boolean {
  return !!id && (node.cat.id === id || node.children.some((c) => containsFolder(c, id)))
}

function folderIcon(cat: Category): ReactElement {
  return isCategoryIconKey(cat.icon) ? <CategoryIcon icon={cat.icon} {...iconProps} /> : <IconFolder {...iconProps} />
}

function sharedMark(cat: Category): ReactElement | null {
  const shared = cat.access !== 'owner' || cat.shared_out
  return shared ? <IconUsers size={14} stroke={1.5} aria-label="Shared" color="var(--mantine-color-dimmed)" /> : null
}

/** One folder, without its subfolders: recent and filtered lists are flat. */
function FlatFolderLink({
  cat,
  description,
  activeFolder,
  onNavigate,
}: { cat: Category; description?: string } & Props): ReactElement {
  return (
    <NavLink
      renderRoot={(props) => <Link {...props} to="/library" search={{ folder: cat.id }} activeOptions={EXACT} />}
      label={cat.name}
      description={description}
      leftSection={folderIcon(cat)}
      rightSection={sharedMark(cat)}
      active={activeFolder === cat.id}
      onClick={onNavigate}
    />
  )
}

function FolderLink({ node, activeFolder, onNavigate }: { node: CatNode } & Props): ReactElement {
  const children = node.children.map((c) => (
    <FolderLink key={c.cat.id} node={c} activeFolder={activeFolder} onNavigate={onNavigate} />
  ))
  return (
    <NavLink
      renderRoot={(props) => <Link {...props} to="/library" search={{ folder: node.cat.id }} activeOptions={EXACT} />}
      label={node.cat.name}
      leftSection={folderIcon(node.cat)}
      rightSection={sharedMark(node.cat)}
      active={activeFolder === node.cat.id}
      defaultOpened={node.children.length > 0 && containsFolder(node, activeFolder)}
      childrenOffset={14}
      onClick={onNavigate}
    >
      {node.children.length > 0 ? children : undefined}
    </NavLink>
  )
}

/**
 * The Folders section. Collapsed, it shows the few most recently updated folders, so a long
 * folder list does not push the rest of the sidebar away; expanded, the whole tree with a filter.
 * Vault folders live on Vaults.
 */
export function SidebarFolders({ activeFolder, onNavigate }: Props): ReactElement {
  const cats = useCategories()
  const [expanded, setExpanded] = useLocalStorage({
    key: 'linkdean-sidebar-folders-expanded',
    defaultValue: false,
    getInitialValueInEffect: false,
  })
  const [query, setQuery] = useState('')
  const unsortedCount = useUnsortedCount().data ?? 0

  const folders = (cats.data || []).filter((c) => !c.is_vault && !c.deleted_at)
  const matches = expanded ? filterFolders(folders, query) : []
  const filtering = expanded && query.trim() !== ''

  const unsorted = (
    <NavLink
      renderRoot={(props) => (
        <Link
          {...props}
          to="/library"
          search={{ folder: FOLDER_UNCATEGORIZED, category: FOLDER_UNCATEGORIZED }}
          activeOptions={EXACT}
        />
      )}
      label="Unsorted"
      leftSection={<IconInbox {...iconProps} />}
      rightSection={
        unsortedCount > 0 ? (
          <Badge size="sm" variant="light" color="gray" radius="sm" aria-label={`${unsortedCount} unsorted`}>
            {unsortedCount > 999 ? '999+' : unsortedCount}
          </Badge>
        ) : undefined
      }
      active={activeFolder === FOLDER_UNCATEGORIZED}
      onClick={onNavigate}
    />
  )

  let body: ReactElement
  if (cats.isLoading) {
    body = (
      <Stack gap={6} px="sm">
        <Skeleton height={20} />
        <Skeleton height={20} />
      </Stack>
    )
  } else if (filtering) {
    body =
      matches.length > 0 ? (
        <>
          {matches.map(({ cat, path }) => (
            <FlatFolderLink
              key={cat.id}
              cat={cat}
              description={path !== cat.name ? path : undefined}
              activeFolder={activeFolder}
              onNavigate={onNavigate}
            />
          ))}
        </>
      ) : (
        <Text size="sm" c="dimmed" px="sm" py={6}>
          No folders match.
        </Text>
      )
  } else if (expanded) {
    body = (
      <>
        {unsorted}
        {buildCategoryTree(folders).map((n) => (
          <FolderLink key={n.cat.id} node={n} activeFolder={activeFolder} onNavigate={onNavigate} />
        ))}
      </>
    )
  } else {
    const recent = recentFolders(folders, activeFolder)
    const hidden = folders.length - recent.length
    body = (
      <>
        {unsorted}
        {recent.map((cat) => (
          <FlatFolderLink key={cat.id} cat={cat} activeFolder={activeFolder} onNavigate={onNavigate} />
        ))}
        {hidden > 0 ? (
          <UnstyledButton className={classes.moreFolders} onClick={() => setExpanded(true)}>
            <Text size="xs" c="dimmed">
              Show all {folders.length} folders
            </Text>
          </UnstyledButton>
        ) : null}
      </>
    )
  }

  return (
    <Stack gap={2}>
      <UnstyledButton
        className={classes.groupToggle}
        aria-expanded={expanded}
        aria-controls="sidebar-folders"
        onClick={() => {
          setExpanded((v) => !v)
          setQuery('')
        }}
      >
        <Group justify="space-between" wrap="nowrap" px="sm" pt="sm" pb={4}>
          <Text size="xs" fw={600} c="dimmed" tt="uppercase" className={classes.groupLabel}>
            Folders
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
        <TextInput
          size="xs"
          mx="sm"
          mb={4}
          type="search"
          name="folder-filter"
          autoComplete="off"
          data-1p-ignore
          data-lpignore="true"
          placeholder="Filter folders"
          aria-label="Filter folders"
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape' && query) {
              e.stopPropagation()
              setQuery('')
            }
          }}
          leftSection={<IconSearch size={14} stroke={1.5} />}
          rightSection={query ? <CloseButton size="sm" aria-label="Clear filter" onClick={() => setQuery('')} /> : null}
        />
      ) : null}
      <Stack gap={2} id="sidebar-folders">
        {body}
      </Stack>
    </Stack>
  )
}
