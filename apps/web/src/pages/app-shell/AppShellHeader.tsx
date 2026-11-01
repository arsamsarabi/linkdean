import { type ReactElement } from 'react'
import { Burger, Button, Code, Group, Indicator, Menu, Skeleton, Title, Tooltip, UnstyledButton } from '@mantine/core'
import {
  IconBooks,
  IconLayoutDashboard,
  IconLock,
  IconLockOpen,
  IconLogout,
  IconSettings,
  IconTrash,
  IconUser,
} from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import {
  BrokenLinksButton,
  DuplicatesButton,
  HistoryButton,
  LogoMark,
  NotificationsBell,
  PageContainer,
  ThemeMenuSwitch,
  UserAvatar,
} from '@/components'
import type { Vault } from '@/schemas'
import { useShellUi } from '@/stores'
import classes from './AppShell.module.css'

const iconProps = { size: 18, stroke: 1.5 } as const
const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  loading: boolean
  landing: string
  onDashboard: boolean
  onLibrary: boolean
  label: string
  displayName: string | undefined
  email: string | undefined
  hasAvatar: boolean
  vaultUnlocked: boolean
  vaults: Vault[]
  globalEnabled: boolean
  onLockVault: (id: string) => void
  onOpenUnlockAll: () => void
  unreadCount: number
  onLogout: () => void
}

export function AppShellHeader({
  loading,
  landing,
  onDashboard,
  onLibrary,
  label,
  displayName,
  email,
  hasAvatar,
  vaultUnlocked,
  vaults,
  globalEnabled,
  onLockVault,
  onOpenUnlockAll,
  unreadCount,
  onLogout,
}: Props): ReactElement {
  const navOpen = useShellUi((s) => s.navOpen)
  const toggleNav = useShellUi((s) => s.toggleNav)
  const openBroken = useShellUi((s) => s.openBroken)
  const openDupes = useShellUi((s) => s.openDupes)
  const openHistory = useShellUi((s) => s.openHistory)
  const openNotif = useShellUi((s) => s.openNotif)

  return (
    <PageContainer h="100%">
      <Group h="100%" justify="space-between">
        <Group gap="md" wrap="nowrap">
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
          <Button
            component={Link}
            to="/"
            variant={onDashboard ? 'light' : 'subtle'}
            leftSection={<IconLayoutDashboard {...iconSm} />}
            size="compact-sm"
            visibleFrom="sm"
          >
            Dashboard
          </Button>
          <Button
            component={Link}
            to="/library"
            variant={onLibrary ? 'light' : 'subtle'}
            leftSection={<IconBooks {...iconSm} />}
            size="compact-sm"
            visibleFrom="sm"
          >
            Library
          </Button>
        </Group>

        {loading ? (
          <Skeleton circle height={36} width={36} />
        ) : (
          <Group gap="xs" wrap="nowrap">
            <DuplicatesButton onClick={openDupes} />
            <BrokenLinksButton onClick={openBroken} />
            <HistoryButton onClick={openHistory} />
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
                {vaults.length > 0 ? <Menu.Divider /> : null}
                {vaults.length > 0 ? <Menu.Label>Vaults</Menu.Label> : null}
                {vaults.map((vault) =>
                  vault.unlocked ? (
                    <Menu.Item
                      key={vault.id}
                      aria-label={`Lock ${vault.name}`}
                      leftSection={<IconLockOpen {...iconProps} />}
                      onClick={() => onLockVault(vault.id)}
                    >
                      {vault.name}
                    </Menu.Item>
                  ) : (
                    <Menu.Item
                      key={vault.id}
                      aria-label={`Unlock ${vault.name} in settings`}
                      component={Link}
                      to="/settings"
                      leftSection={<IconLock {...iconProps} />}
                    >
                      {vault.name}
                    </Menu.Item>
                  )
                )}
                {globalEnabled ? (
                  <Menu.Item leftSection={<IconLockOpen {...iconProps} />} onClick={onOpenUnlockAll}>
                    Unlock all
                  </Menu.Item>
                ) : null}
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
    </PageContainer>
  )
}
