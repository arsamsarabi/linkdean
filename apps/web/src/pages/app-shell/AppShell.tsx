import { useEffect, type ReactElement } from 'react'
import { Drawer, Group, Loader, AppShell as MantineAppShell, NavLink, Stack } from '@mantine/core'
import { useHotkeys } from '@mantine/hooks'
import { IconBooks, IconLayoutDashboard } from '@tabler/icons-react'
import { Link, Outlet, useNavigate, useRouterState } from '@tanstack/react-router'
import {
  BrokenLinksDrawer,
  CommandPalette,
  HistoryDrawer,
  NotificationsDrawer,
  PageContainer,
  QueryErrorAlert,
} from '@/components'
import { useLogout, usePrefsHydration, useUnreadNotificationCount } from '@/hooks'
import { resolveDefaultLanding } from '@/lib'
import { useShellUi, useUi } from '@/stores'
import { AppShellHeader } from './AppShellHeader'

const iconProps = { size: 18, stroke: 1.5 } as const

export function AppShell(): ReactElement {
  const nav = useNavigate()
  const me = usePrefsHydration(true)
  const logout = useLogout()
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const shell = useShellUi()
  const unread = useUnreadNotificationCount(!!me.data)

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
        shell.openPalette()
      },
    ],
    [
      'mod+K',
      (e) => {
        e.preventDefault()
        shell.togglePalette()
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
        <AppShellHeader
          loading={loading}
          landing={landing}
          onDashboard={onDashboard}
          onLibrary={onLibrary}
          label={label}
          displayName={user?.display_name}
          email={user?.email}
          hasAvatar={!!user?.has_avatar}
          vaultUnlocked={vaultUnlocked}
          unreadCount={unread.data?.count ?? 0}
          onLogout={() => void onLogout()}
        />
      </MantineAppShell.Header>

      <MantineAppShell.Main>
        <PageContainer pb="xl" fadeIn>
          {me.isError ? (
            <QueryErrorAlert
              error={me.error}
              onRetry={() => void me.refetch()}
              title="Failed to load account"
            />
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

      <Drawer
        opened={shell.navOpen}
        onClose={shell.closeNav}
        size="xs"
        padding="md"
        title="Navigate"
        hiddenFrom="sm"
      >
        <Stack gap="xs">
          <NavLink
            component={Link}
            to="/"
            label="Dashboard"
            leftSection={<IconLayoutDashboard {...iconProps} />}
            active={onDashboard}
            onClick={shell.closeNav}
          />
          <NavLink
            component={Link}
            to="/library"
            label="Library"
            leftSection={<IconBooks {...iconProps} />}
            active={onLibrary}
            onClick={shell.closeNav}
          />
        </Stack>
      </Drawer>

      <CommandPalette opened={shell.paletteOpen} onClose={shell.closePalette} />
      <BrokenLinksDrawer opened={shell.brokenOpen} onClose={shell.closeBroken} />
      <HistoryDrawer opened={shell.historyOpen} onClose={shell.closeHistory} />
      <NotificationsDrawer opened={shell.notifOpen} onClose={shell.closeNotif} />
    </MantineAppShell>
  )
}
