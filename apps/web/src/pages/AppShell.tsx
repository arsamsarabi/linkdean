import { useEffect, type ReactElement } from 'react'
import { Outlet, Link, useNavigate } from '@tanstack/react-router'
import {
  AppShell as MantineAppShell,
  Group,
  Indicator,
  Loader,
  Menu,
  Skeleton,
  Title,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import { IconLogout, IconSettings, IconUser } from '@tabler/icons-react'
import { PageContainer } from '../components/PageContainer'
import { LogoMark } from '../components/LogoMark'
import { QueryErrorAlert } from '../components/QueryErrorAlert'
import { ThemeMenuSwitch } from '../components/ThemeMenuSwitch'
import { UserAvatar } from '../components/UserAvatar'
import { useLogout } from '../hooks/useAuth'
import { usePrefsHydration } from '../hooks/usePrefs'
import { useUi } from '../store'

const iconProps = { size: 18, stroke: 1.5 } as const

export function AppShell(): ReactElement {
  const nav = useNavigate()
  const me = usePrefsHydration(true)
  const logout = useLogout()
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)

  useEffect(() => {
    if (me.data && !me.data.user.onboarding_completed) {
      void nav({ to: '/onboarding' })
    }
  }, [me.data, nav])

  async function onLogout(): Promise<void> {
    await logout.mutateAsync()
    void nav({ to: '/login' })
  }

  const user = me.data?.user
  const label = user?.display_name || user?.email || '?'
  const loading = me.isLoading && !me.data

  return (
    <MantineAppShell header={{ height: 64 }} padding="lg">
      <MantineAppShell.Header px={0}>
        <PageContainer h="100%">
          <Group h="100%" justify="space-between">
            <UnstyledButton component={Link} to="/library" aria-label="Library">
              <Group gap="sm" wrap="nowrap">
                <LogoMark size={28} />
                <Title order={3} c="violet">
                  Linkdean
                </Title>
              </Group>
            </UnstyledButton>

            {loading ? (
              <Skeleton circle height={36} width={36} />
            ) : (
              <Menu shadow="md" width={240} position="bottom-end">
                <Menu.Target>
                  <Tooltip label="Account" position="bottom">
                    <UnstyledButton aria-label="Account menu">
                      <Indicator
                        disabled={!vaultUnlocked}
                        color="green"
                        size={10}
                        offset={4}
                        processing
                        withBorder
                      >
                        <UserAvatar hasAvatar={!!user?.has_avatar} label={label} alt={label} />
                      </Indicator>
                    </UnstyledButton>
                  </Tooltip>
                </Menu.Target>
                <Menu.Dropdown>
                  <Menu.Label>{user?.display_name || user?.email || 'User'}</Menu.Label>
                  {vaultUnlocked ? <Menu.Label c="green">Vault unlocked</Menu.Label> : null}
                  <Menu.Item
                    component={Link}
                    to="/profile"
                    leftSection={<IconUser {...iconProps} />}
                  >
                    Profile
                  </Menu.Item>
                  <Menu.Item
                    component={Link}
                    to="/settings"
                    leftSection={<IconSettings {...iconProps} />}
                  >
                    Settings
                  </Menu.Item>
                  <Menu.Divider />
                  <ThemeMenuSwitch />
                  <Menu.Divider />
                  <Menu.Item
                    color="red"
                    onClick={() => void onLogout()}
                    leftSection={<IconLogout {...iconProps} />}
                  >
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
    </MantineAppShell>
  )
}
