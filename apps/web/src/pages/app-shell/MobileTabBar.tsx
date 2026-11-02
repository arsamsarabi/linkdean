import { type ReactElement } from 'react'
import { Group, Text, UnstyledButton } from '@mantine/core'
import { IconBookmarkPlus, IconBooks, IconLayoutDashboard, IconSearch, IconUser } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { buildLibrarySearch } from '@/lib'
import { useShellUi } from '@/stores'
import classes from './AppShell.module.css'
import { EXACT } from './sidebarLinks'

const iconProps = { size: 22, stroke: 1.5 } as const

type Props = {
  pathname: string
  /** The library folder open now, so Add files the new bookmark there. */
  folder: string | undefined
}

function Tab({ label, icon, active }: { label: string; icon: ReactElement; active?: boolean }): ReactElement {
  return (
    <>
      {icon}
      <Text size="xs" fw={active ? 600 : 400}>
        {label}
      </Text>
    </>
  )
}

/** Phone navigation: the five things done most, under the thumb. The sidebar has the rest. */
export function MobileTabBar({ pathname, folder }: Props): ReactElement {
  const openPalette = useShellUi((s) => s.openPalette)
  const onLibrary = pathname.startsWith('/library')
  return (
    <Group component="nav" aria-label="Tabs" h="100%" grow gap={0} className={classes.tabBar}>
      <UnstyledButton
        renderRoot={(props) => <Link {...props} to="/" activeOptions={EXACT} />}
        className={classes.tab}
        data-active={pathname === '/' || undefined}
      >
        <Tab label="Home" icon={<IconLayoutDashboard {...iconProps} />} active={pathname === '/'} />
      </UnstyledButton>
      <UnstyledButton
        renderRoot={(props) => <Link {...props} to="/library" activeOptions={EXACT} />}
        className={classes.tab}
        data-active={onLibrary || undefined}
      >
        <Tab label="Library" icon={<IconBooks {...iconProps} />} active={onLibrary} />
      </UnstyledButton>
      <UnstyledButton
        renderRoot={(props) => (
          <Link {...props} to="/library" search={buildLibrarySearch({ create: true, folder })} activeOptions={EXACT} />
        )}
        className={`${classes.tab} ${classes.tabAdd}`}
      >
        <Tab label="Add" icon={<IconBookmarkPlus {...iconProps} />} />
      </UnstyledButton>
      <UnstyledButton className={classes.tab} onClick={openPalette}>
        <Tab label="Search" icon={<IconSearch {...iconProps} />} />
      </UnstyledButton>
      <UnstyledButton
        renderRoot={(props) => <Link {...props} to="/profile" activeOptions={EXACT} />}
        className={classes.tab}
        data-active={pathname === '/profile' || undefined}
      >
        <Tab label="Me" icon={<IconUser {...iconProps} />} active={pathname === '/profile'} />
      </UnstyledButton>
    </Group>
  )
}
