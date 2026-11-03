import { useEffect, type ReactElement } from 'react'
import { Group, Loader, AppShell as MantineAppShell } from '@mantine/core'
import { useHotkeys } from '@mantine/hooks'
import { notifications } from '@mantine/notifications'
import { Outlet, useNavigate, useRouterState } from '@tanstack/react-router'
import {
  BrokenLinksDrawer,
  CommandPalette,
  DuplicateMergeDrawer,
  HistoryDrawer,
  NotificationsDrawer,
  PageContainer,
  QueryErrorAlert,
} from '@/components'
import {
  useLogout,
  usePasteToAdd,
  usePrefsHydration,
  useSessionSync,
  useShareInvites,
  useUnreadNotificationCount,
} from '@/hooks'
import { resolveDefaultLanding } from '@/lib'
import { useShellUi, useUi } from '@/stores'
import { AppShellHeader } from './AppShellHeader'
import { AppSidebar } from './AppSidebar'
import { MobileTabBar } from './MobileTabBar'

export function AppShell(): ReactElement {
  const nav = useNavigate()
  const me = usePrefsHydration(true)
  const logout = useLogout()
  useSessionSync()
  usePasteToAdd(!!me.data)
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const shell = useShellUi()
  const unread = useUnreadNotificationCount(!!me.data)
  const invites = useShareInvites(!!me.data)
  const pendingInvites = invites.data?.invites.filter((i) => !i.rejected_at).length ?? 0

  useEffect(() => {
    if (me.data && !me.data.user.onboarding_completed) {
      void nav({ to: '/onboarding' })
    }
  }, [me.data, nav])

  useHotkeys([
    ['mod+B', () => shell.toggleNavCollapsed()],
    [
      'mod+K',
      (e) => {
        e.preventDefault()
        shell.togglePalette()
      },
    ],
  ])

  async function onLogout(): Promise<void> {
    try {
      await logout.mutateAsync()
    } catch {
      notifications.show({ color: 'red', message: 'Could not sign out. Try again.' })
      return
    }
    void nav({ to: '/login' })
  }

  const user = me.data?.user
  const label = user?.display_name || user?.email || '?'
  const loading = me.isLoading && !me.data
  const pathname = useRouterState({ select: (s) => s.location.pathname })
  const search = useRouterState({ select: (s) => s.location.search as Record<string, unknown> })
  const landing = resolveDefaultLanding(me.data?.settings.default_landing)

  return (
    <MantineAppShell
      header={{ height: 64 }}
      navbar={{ width: 248, breakpoint: 'sm', collapsed: { mobile: !shell.navOpen, desktop: shell.navCollapsed } }}
      footer={{ height: { base: 64, sm: 0 } }}
      padding="lg"
    >
      <MantineAppShell.Header px={0}>
        <AppShellHeader
          loading={loading}
          landing={landing}
          label={label}
          displayName={user?.display_name ?? undefined}
          email={user?.email}
          hasAvatar={!!user?.has_avatar}
          vaultUnlocked={vaultUnlocked}
          // A new invite is also an unread notification, so don't add the two.
          unreadCount={Math.max(unread.data?.count ?? 0, pendingInvites)}
          onLogout={() => void onLogout()}
        />
      </MantineAppShell.Header>

      <MantineAppShell.Navbar aria-label="Main">
        <AppSidebar pathname={pathname} search={search} vaultUnlocked={vaultUnlocked} />
      </MantineAppShell.Navbar>

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

      <MantineAppShell.Footer hiddenFrom="sm">
        <MobileTabBar
          pathname={pathname}
          folder={pathname.startsWith('/library') && typeof search.folder === 'string' ? search.folder : undefined}
        />
      </MantineAppShell.Footer>

      <CommandPalette opened={shell.paletteOpen} onClose={shell.closePalette} />
      <BrokenLinksDrawer opened={shell.brokenOpen} onClose={shell.closeBroken} />
      <DuplicateMergeDrawer opened={shell.dupesOpen} onClose={shell.closeDupes} />
      <HistoryDrawer opened={shell.historyOpen} onClose={shell.closeHistory} />
      <NotificationsDrawer opened={shell.notifOpen} onClose={shell.closeNotif} />
    </MantineAppShell>
  )
}
