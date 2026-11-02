import { type ReactElement } from 'react'
import {
  Box,
  Burger,
  Button,
  Code,
  Group,
  Indicator,
  Kbd,
  Menu,
  Skeleton,
  Text,
  Title,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import { IconBookmarkPlus, IconLogout, IconSearch, IconSettings, IconTrash, IconUser } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { LogoMark, NotificationsBell, ThemeMenuSwitch, UserAvatar } from '@/components'
import { buildLibrarySearch } from '@/lib'
import { useShellUi } from '@/stores'
import classes from './AppShell.module.css'

const iconProps = { size: 18, stroke: 1.5 } as const

type Props = {
  loading: boolean
  landing: string
  label: string
  displayName: string | undefined
  email: string | undefined
  hasAvatar: boolean
  vaultUnlocked: boolean
  unreadCount: number
  onLogout: () => void
}

export function AppShellHeader({
  loading,
  landing,
  label,
  displayName,
  email,
  hasAvatar,
  vaultUnlocked,
  unreadCount,
  onLogout,
}: Props): ReactElement {
  const navOpen = useShellUi((s) => s.navOpen)
  const toggleNav = useShellUi((s) => s.toggleNav)
  const openPalette = useShellUi((s) => s.openPalette)
  const openNotif = useShellUi((s) => s.openNotif)

  return (
    <Group h="100%" px="md" justify="space-between" wrap="nowrap" gap="md">
      <Group gap="md" wrap="nowrap" className={classes.brand}>
        <Burger opened={navOpen} onClick={toggleNav} hiddenFrom="sm" size="sm" aria-label="Open navigation" />
        <UnstyledButton
          component={Link}
          to={landing}
          aria-label={landing === '/library' ? 'Go to library' : 'Go to dashboard'}
        >
          <Group gap="sm" wrap="nowrap">
            <LogoMark size={28} />
            <Title order={3} c="violet" visibleFrom="xs">
              Linkdean
            </Title>
          </Group>
        </UnstyledButton>
      </Group>

      {/* Looks like a search field; opens the command palette, which searches, adds and jumps. */}
      <UnstyledButton onClick={openPalette} className={classes.searchTrigger} visibleFrom="sm" aria-label="Search">
        <IconSearch size={16} stroke={1.5} />
        <Text size="sm" c="dimmed" className={classes.searchText}>
          Search, add or jump to a folder…
        </Text>
        <Kbd size="xs">⌘K</Kbd>
      </UnstyledButton>

      {loading ? (
        <Skeleton circle height={36} width={36} />
      ) : (
        <Group gap="xs" wrap="nowrap">
          <Box visibleFrom="sm">
            <Tooltip label="New bookmark (N in the library)" position="bottom">
              <Button
                renderRoot={(props) => <Link {...props} to="/library" search={buildLibrarySearch({ create: true })} />}
                leftSection={<IconBookmarkPlus size={16} stroke={1.5} />}
              >
                Add
              </Button>
            </Tooltip>
          </Box>
          <Indicator
            disabled={unreadCount < 1}
            label={unreadCount > 9 ? '9+' : unreadCount}
            size={16}
            color="violet"
            offset={4}
          >
            <NotificationsBell unread={unreadCount} onClick={openNotif} />
          </Indicator>
          <Menu shadow="md" width={240} position="bottom-end">
            <Menu.Target>
              <Tooltip label="Account" position="bottom">
                <UnstyledButton aria-label="Account menu">
                  <Indicator disabled={!vaultUnlocked} color="green" size={10} offset={4} processing withBorder>
                    <UserAvatar hasAvatar={hasAvatar} label={label} alt={label} />
                  </Indicator>
                </UnstyledButton>
              </Tooltip>
            </Menu.Target>
            <Menu.Dropdown>
              <Menu.Label>{displayName || email || 'User'}</Menu.Label>
              <Menu.Divider />
              <Menu.Label>
                <Group gap={6} align="center" justify="center">
                  <Code classNames={{ root: classes.codeRoot }}>⌘K</Code>
                  Command palette
                </Group>
              </Menu.Label>
              <Menu.Divider />
              <Menu.Item component={Link} to="/profile" leftSection={<IconUser {...iconProps} />}>
                Profile
              </Menu.Item>
              <Menu.Item component={Link} to="/settings" leftSection={<IconSettings {...iconProps} />}>
                Config options
              </Menu.Item>
              <Menu.Item component={Link} to="/trash" leftSection={<IconTrash {...iconProps} />}>
                Deleted items
              </Menu.Item>
              <Menu.Divider />
              <ThemeMenuSwitch />
              <Menu.Divider />
              <Menu.Item color="red" onClick={onLogout} leftSection={<IconLogout {...iconProps} />}>
                Logout
              </Menu.Item>
            </Menu.Dropdown>
          </Menu>
        </Group>
      )}
    </Group>
  )
}
