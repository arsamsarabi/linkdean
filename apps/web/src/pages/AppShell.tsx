import { useEffect } from 'react'
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
import {
  IconLogout,
  IconSettings,
  IconUser,
} from '@tabler/icons-react'
import { api, setToken } from '../api'
import { PageContainer } from '../components/PageContainer'
import { ThemeMenuSwitch } from '../components/ThemeMenuSwitch'
import { UserAvatar } from '../components/UserAvatar'
import { usePrefsHydration } from '../hooks/usePrefs'
import { useUi } from '../store'

const iconProps = { size: 18, stroke: 1.5 } as const

export function AppShell() {
  const nav = useNavigate()
  const me = usePrefsHydration(true)
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)

  useEffect(() => {
    if (me.data && !me.data.user.onboarding_completed) {
      nav({ to: '/onboarding' })
    }
  }, [me.data, nav])

  async function logout() {
    await api.logout().catch(() => {})
    setToken(null)
    nav({ to: '/login' })
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
                <img
                  src="/linkdean-symbol.svg"
                  alt=""
                  width={28}
                  height={28}
                  style={{ display: 'block' }}
                />
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
                        <UserAvatar
                          hasAvatar={!!user?.has_avatar}
                          label={label}
                          alt={label}
                        />
                      </Indicator>
                    </UnstyledButton>
                  </Tooltip>
                </Menu.Target>
                <Menu.Dropdown>
                  <Menu.Label>{user?.display_name || user?.email || 'User'}</Menu.Label>
                  {vaultUnlocked ? (
                    <Menu.Label c="green">Vault unlocked</Menu.Label>
                  ) : null}
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
                    onClick={logout}
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
        <PageContainer pb="xl" className="ld-fade-in">
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
