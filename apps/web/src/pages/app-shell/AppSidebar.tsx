import { type ReactElement, type ReactNode } from 'react'
import { AppShell, Badge, NavLink, ScrollArea, Stack, Text } from '@mantine/core'
import {
  IconBookmark,
  IconBooks,
  IconCopy,
  IconHistory,
  IconLayoutDashboard,
  IconLinkOff,
  IconLock,
  IconPin,
  IconSettings,
  IconStar,
  IconTrash,
  IconUsers,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { usePrefetchBrokenLinks, useShareInvites } from '@/hooks'
import { asStringList, type FlagKey } from '@/lib'
import { useShellUi } from '@/stores'
import classes from './AppShell.module.css'
import { SidebarFolders } from './SidebarFolders'
import { EXACT } from './sidebarLinks'

const iconProps = { size: 18, stroke: 1.5 } as const

const QUICK_VIEWS: { flag: FlagKey; label: string; Icon: typeof IconStar }[] = [
  { flag: 'unread', label: 'Reading list', Icon: IconBookmark },
  { flag: 'favorite', label: 'Favorites', Icon: IconStar },
  { flag: 'pinned', label: 'Pinned', Icon: IconPin },
]

type Props = {
  pathname: string
  search: Record<string, unknown>
  vaultUnlocked: boolean
}

function Group({ label, children }: { label: string; children: ReactNode }): ReactElement {
  return (
    <Stack gap={2}>
      <Text size="xs" fw={600} c="dimmed" tt="uppercase" px="sm" pt="sm" pb={4} className={classes.groupLabel}>
        {label}
      </Text>
      {children}
    </Stack>
  )
}

/**
 * Primary navigation. A fixed column on wide screens; on phones the burger slides it in, and
 * any choice closes it again.
 */
export function AppSidebar({ pathname, search, vaultUnlocked }: Props): ReactElement {
  const closeNav = useShellUi((s) => s.closeNav)
  const openBroken = useShellUi((s) => s.openBroken)
  const openDupes = useShellUi((s) => s.openDupes)
  const openHistory = useShellUi((s) => s.openHistory)
  const prefetchBroken = usePrefetchBrokenLinks()
  const invites = useShareInvites()
  const pendingInvites = (invites.data?.invites ?? []).filter((i) => !i.rejected_at).length

  const onLibrary = pathname.startsWith('/library')
  const flags = onLibrary ? asStringList(search.flag) : []
  const folder = onLibrary && typeof search.folder === 'string' && search.folder !== 'all' ? search.folder : undefined
  const plainLibrary = onLibrary && flags.length === 0 && !folder
  const open = (fn: () => void) => () => {
    closeNav()
    fn()
  }

  return (
    <>
      <AppShell.Section grow component={ScrollArea} px="sm" py="md">
        <Stack gap={2}>
          <NavLink
            component={Link}
            activeOptions={EXACT}
            to="/"
            label="Dashboard"
            leftSection={<IconLayoutDashboard {...iconProps} />}
            active={pathname === '/'}
            onClick={closeNav}
          />
          <NavLink
            component={Link}
            activeOptions={EXACT}
            to="/library"
            label="Library"
            leftSection={<IconBooks {...iconProps} />}
            active={plainLibrary}
            onClick={closeNav}
          />
          <NavLink
            component={Link}
            activeOptions={EXACT}
            to="/shared"
            label="Shared"
            leftSection={<IconUsers {...iconProps} />}
            rightSection={
              pendingInvites > 0 ? (
                <Badge size="sm" circle aria-label={`${pendingInvites} invites waiting`}>
                  {pendingInvites}
                </Badge>
              ) : null
            }
            active={pathname === '/shared'}
            onClick={closeNav}
          />
          {vaultUnlocked ? (
            <NavLink
              component={Link}
              activeOptions={EXACT}
              to="/vaults"
              label="Vaults"
              leftSection={<IconLock {...iconProps} />}
              active={pathname === '/vaults' || pathname.startsWith('/vaults/')}
              onClick={closeNav}
            />
          ) : null}
        </Stack>

        <Group label="Quick views">
          {QUICK_VIEWS.map(({ flag, label, Icon }) => (
            <NavLink
              key={flag}
              renderRoot={(props) => <Link {...props} to="/library" search={{ flag }} activeOptions={EXACT} />}
              label={label}
              leftSection={<Icon {...iconProps} />}
              active={flags.length === 1 && flags[0] === flag}
              onClick={closeNav}
            />
          ))}
        </Group>

        <SidebarFolders activeFolder={folder} onNavigate={closeNav} />

        <Group label="Library health">
          <NavLink
            component="button"
            label="Broken links"
            leftSection={<IconLinkOff {...iconProps} />}
            onClick={open(openBroken)}
            onMouseEnter={prefetchBroken}
            onFocus={prefetchBroken}
          />
          <NavLink
            component="button"
            label="Duplicates"
            leftSection={<IconCopy {...iconProps} />}
            onClick={open(openDupes)}
          />
          <NavLink
            component="button"
            label="History"
            leftSection={<IconHistory {...iconProps} />}
            onClick={open(openHistory)}
          />
        </Group>
      </AppShell.Section>

      <AppShell.Section px="sm" py="sm" className={classes.footer}>
        <Stack gap={2}>
          <NavLink
            component={Link}
            activeOptions={EXACT}
            to="/trash"
            label="Deleted items"
            leftSection={<IconTrash {...iconProps} />}
            active={pathname === '/trash'}
            onClick={closeNav}
          />
          <NavLink
            component={Link}
            activeOptions={EXACT}
            to="/settings"
            label="Config options"
            leftSection={<IconSettings {...iconProps} />}
            active={pathname === '/settings'}
            onClick={closeNav}
          />
        </Stack>
      </AppShell.Section>
    </>
  )
}
