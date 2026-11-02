import { type ReactElement } from 'react'
import { NavLink, Skeleton, Stack } from '@mantine/core'
import { IconFolder, IconInbox, IconUsers } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { useCategories } from '@/hooks'
import { buildCategoryTree, CategoryIcon, isCategoryIconKey, type CatNode } from '@/lib'
import { FOLDER_UNCATEGORIZED } from '@/services'
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

function FolderLink({ node, activeFolder, onNavigate }: { node: CatNode } & Props): ReactElement {
  const shared = node.cat.access !== 'owner' || node.cat.shared_out
  const children = node.children.map((c) => (
    <FolderLink key={c.cat.id} node={c} activeFolder={activeFolder} onNavigate={onNavigate} />
  ))
  return (
    <NavLink
      renderRoot={(props) => <Link {...props} to="/library" search={{ folder: node.cat.id }} activeOptions={EXACT} />}
      label={node.cat.name}
      leftSection={
        isCategoryIconKey(node.cat.icon) ? (
          <CategoryIcon icon={node.cat.icon} {...iconProps} />
        ) : (
          <IconFolder {...iconProps} />
        )
      }
      rightSection={
        shared ? <IconUsers size={14} stroke={1.5} aria-label="Shared" color="var(--mantine-color-dimmed)" /> : null
      }
      active={activeFolder === node.cat.id}
      defaultOpened={node.children.length > 0 && containsFolder(node, activeFolder)}
      childrenOffset={14}
      onClick={onNavigate}
    >
      {node.children.length > 0 ? children : undefined}
    </NavLink>
  )
}

/** The folder tree, so any folder is one click away from any page. Vault folders live on Vaults. */
export function SidebarFolders({ activeFolder, onNavigate }: Props): ReactElement {
  const cats = useCategories()
  if (cats.isLoading) {
    return (
      <Stack gap={6} px="sm">
        <Skeleton height={20} />
        <Skeleton height={20} />
      </Stack>
    )
  }
  const tree = buildCategoryTree((cats.data || []).filter((c) => !c.is_vault && !c.deleted_at))
  return (
    <Stack gap={2}>
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
        active={activeFolder === FOLDER_UNCATEGORIZED}
        onClick={onNavigate}
      />
      {tree.map((n) => (
        <FolderLink key={n.cat.id} node={n} activeFolder={activeFolder} onNavigate={onNavigate} />
      ))}
    </Stack>
  )
}
