import { type ReactElement } from 'react'
import { AppShell, Indicator, ScrollArea, Stack, Tooltip } from '@mantine/core'
import {
  IconBooks,
  IconFolder,
  IconFolders,
  IconLayoutDashboard,
  IconLock,
  IconSettings,
  IconShield,
  IconTrash,
  IconUsers,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { useCategories, useShareInvites } from '@/hooks'
import { asStringList, CategoryIcon, isCategoryIconKey } from '@/lib'
import type { Category } from '@/schemas'
import { useShellUi } from '@/stores'
import classes from './AppShell.module.css'
import { recentFolders } from './folderLists'
import { EXACT } from './sidebarLinks'

const iconProps = { size: 18, stroke: 1.5 } as const

type Props = {
  pathname: string
  search: Record<string, unknown>
  vaultUnlocked: boolean
  isAdmin: boolean
}

function folderIcon(cat: Category): ReactElement {
  return isCategoryIconKey(cat.icon) ? <CategoryIcon icon={cat.icon} {...iconProps} /> : <IconFolder {...iconProps} />
}

function RailLink({
  label,
  to,
  search,
  icon,
  active,
  onClick,
  indicator,
}: {
  label: string
  to: string
  search?: Record<string, string>
  icon: ReactElement
  active: boolean
  onClick: () => void
  indicator?: boolean
}): ReactElement {
  const section = indicator ? (
    <Indicator size={8} offset={2} color="violet" processing={false} withBorder={false}>
      {icon}
    </Indicator>
  ) : (
    icon
  )
  return (
    <Tooltip label={label} position="right" openDelay={200} withArrow>
      <Link
        to={to}
        search={search}
        activeOptions={EXACT}
        className={classes.railItem}
        data-active={active || undefined}
        aria-current={active ? 'page' : undefined}
        aria-label={label}
        onClick={onClick}
      >
        {section}
      </Link>
    </Tooltip>
  )
}

/**
 * Narrow icon rail for sm+ when the sidebar is collapsed. Tooltips carry the labels;
 * folder tree and group headings stay on the expanded sidebar.
 */
export function SidebarRail({ pathname, search, vaultUnlocked, isAdmin }: Props): ReactElement {
  const closeNav = useShellUi((s) => s.closeNav)
  const invites = useShareInvites()
  const pendingInvites = (invites.data?.invites ?? []).filter((i) => !i.rejected_at).length
  const cats = useCategories()

  const onLibrary = pathname.startsWith('/library')
  const flags = onLibrary ? asStringList(search.flag) : []
  const folder = onLibrary && typeof search.folder === 'string' && search.folder !== 'all' ? search.folder : undefined
  const plainLibrary = onLibrary && flags.length === 0 && !folder

  const folders = (cats.data || []).filter((c) => !c.is_vault && !c.deleted_at)
  const recent = recentFolders(folders, folder)

  return (
    <>
      <AppShell.Section grow component={ScrollArea} py="md" className={classes.rail}>
        <Stack gap={4} align="center">
          <RailLink
            label="Dashboard"
            to="/"
            icon={<IconLayoutDashboard {...iconProps} />}
            active={pathname === '/'}
            onClick={closeNav}
          />
          <RailLink
            label="Library"
            to="/library"
            icon={<IconBooks {...iconProps} />}
            active={!!plainLibrary}
            onClick={closeNav}
          />
          <RailLink
            label="Shared"
            to="/shared"
            icon={<IconUsers {...iconProps} />}
            active={pathname === '/shared'}
            onClick={closeNav}
            indicator={pendingInvites > 0}
          />
          <RailLink
            label="Categories"
            to="/categories"
            icon={<IconFolders {...iconProps} />}
            active={pathname === '/categories'}
            onClick={closeNav}
          />
          {vaultUnlocked ? (
            <RailLink
              label="Vaults"
              to="/vaults"
              icon={<IconLock {...iconProps} />}
              active={pathname === '/vaults' || pathname.startsWith('/vaults/')}
              onClick={closeNav}
            />
          ) : null}
          {recent.length > 0 ? <div className={classes.railDivider} role="separator" /> : null}
          {recent.map((cat) => (
            <RailLink
              key={cat.id}
              label={cat.name}
              to="/library"
              search={{ folder: cat.id }}
              icon={folderIcon(cat)}
              active={folder === cat.id}
              onClick={closeNav}
            />
          ))}
        </Stack>
      </AppShell.Section>

      <AppShell.Section py="sm" className={`${classes.footer} ${classes.rail}`}>
        <Stack gap={4} align="center">
          <RailLink
            label="Deleted items"
            to="/trash"
            icon={<IconTrash {...iconProps} />}
            active={pathname === '/trash'}
            onClick={closeNav}
          />
          <RailLink
            label="Config options"
            to="/settings"
            icon={<IconSettings {...iconProps} />}
            active={pathname === '/settings'}
            onClick={closeNav}
          />
          {isAdmin ? (
            <RailLink
              label="Admin"
              to="/admin"
              icon={<IconShield {...iconProps} />}
              active={pathname === '/admin'}
              onClick={closeNav}
            />
          ) : null}
        </Stack>
      </AppShell.Section>
    </>
  )
}
