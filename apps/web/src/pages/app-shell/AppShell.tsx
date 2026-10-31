import { useEffect, type ReactElement } from 'react'
import {
  Button,
  Code,
  Group,
  Indicator,
  Loader,
  AppShell as MantineAppShell,
  Menu,
  Skeleton,
  Title,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import { useDisclosure, useHotkeys } from '@mantine/hooks'
import {
  IconBooks,
  IconLayoutDashboard,
  IconLogout,
  IconSettings,
  IconTrash,
  IconUser,
} from '@tabler/icons-react'
import { Link, Outlet, useNavigate, useRouterState } from '@tanstack/react-router'
import {
  CommandPalette,
  LogoMark,
  PageContainer,
  QueryErrorAlert,
  ThemeMenuSwitch,
  UserAvatar,
} from '@/components'
import { useLogout, usePrefsHydration } from '@/hooks'
import { resolveDefaultLanding } from '@/lib'
import { useUi } from '@/stores'
import classes from './AppShell.module.css'

const iconProps = { size: 18, stroke: 1.5 } as const
const iconSm = { size: 16, stroke: 1.5 } as const

export function AppShell(): ReactElement {
  const nav = useNavigate()
  const me = usePrefsHydration(true)
  const logout = useLogout()
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const [paletteOpen, { open: openPalette, close: closePalette, toggle: togglePalette }] = useDisclosure(false)

  useEffect(() => {
    if (me.data && !me.data.user.onboarding_completed) {
      void nav({ to: '/onboarding' })
    }
  }, [me.data, nav])

  useHotkeys([
    [
      '/',
      (e) => {
        const t = e.target as HTMLElement | null
        if (t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA' || t?.isContentEditable) return
        e.preventDefault()
        openPalette()
      },
    ],
    [
      'mod+K',
      (e) => {
        e.preventDefault()
        togglePalette()
      },
    ],
  ])

  async function onLogout(): Promise<void> {
    await logout.mutateAsync()
    void nav({ to: '/login' })
  }

  const user = me.data?.user
  const label = user?.display_name || user?.email || '?'
  const loading = me.isLoading && !me.data
  const pathname = useRouterState({ select: (s) => s.location.pathname })
  const landing = resolveDefaultLanding(me.data?.settings.default_landing)
  const onDashboard = pathname === '/'
  const onLibrary = pathname.startsWith('/library')

  return (
    <MantineAppShell header={{ height: 64 }} padding="lg">
      <MantineAppShell.Header px={0}>
        <PageContainer h="100%">
          <Group h="100%" justify="space-between">
            <Group gap="md" wrap="nowrap">
              <UnstyledButton
                component={Link}
                to={landing}
                aria-label={landing === '/library' ? 'Go to library' : 'Go to dashboard'}
              >
                <Group gap="sm" wrap="nowrap">
                  <LogoMark size={28} />
                  <Title order={3} c="violet">
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
              >
                Dashboard
              </Button>
              <Button
                component={Link}
                to="/library"
                variant={onLibrary ? 'light' : 'subtle'}
                leftSection={<IconBooks {...iconSm} />}
                size="compact-sm"
              >
                Library
              </Button>
            </Group>

            {loading ? (
              <Skeleton circle height={36} width={36} />
            ) : (
              <Menu shadow="md" width={240} position="bottom-end">
                <Menu.Target>
                  <Tooltip label="Account" position="bottom">
                    <UnstyledButton aria-label="Account menu">
                      <Indicator disabled={!vaultUnlocked} color="green" size={10} offset={4} processing withBorder>
                        <UserAvatar hasAvatar={!!user?.has_avatar} label={label} alt={label} />
                      </Indicator>
                    </UnstyledButton>
                  </Tooltip>
                </Menu.Target>
                <Menu.Dropdown>
                  <Menu.Label>{user?.display_name || user?.email || 'User'}</Menu.Label>
                  {vaultUnlocked ? <Menu.Label c="green">Vault unlocked</Menu.Label> : null}
                  <Menu.Divider />
                  <Menu.Label>
                    <Group gap={6} align="center" justify="center">
                      <Code classNames={{root: classes.codeRoot}}>
                        ⌘K
                      </Code>
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
                  <Menu.Item color="red" onClick={() => void onLogout()} leftSection={<IconLogout {...iconProps} />}>
                    Logout
                  </Menu.Item>
                </Menu.Dropdown>
              </Menu>
            )}
          </Group>
        </PageContainer>
      </MantineAppShell.Header>

      <MantineAppShell.Main>
        <PageContainer pb="xl" fadeIn>
          {me.isError ? (
            <QueryErrorAlert error={me.error} onRetry={() => void me.refetch()} title="Failed to load account" />
          ) : null}
          {loading ? (
            <Group justify="center" py={80}>
              <Loader color="violet" type="dots" />
            </Group>
          ) : (
            <Outlet />
          )}
        </PageContainer>
      </MantineAppShell.Main>
      <CommandPalette opened={paletteOpen} onClose={closePalette} />
    </MantineAppShell>
  )
}
