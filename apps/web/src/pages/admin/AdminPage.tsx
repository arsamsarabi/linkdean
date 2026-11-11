import { type ReactElement } from 'react'
import { Group, Stack, Tabs, ThemeIcon, Title } from '@mantine/core'
import { useMediaQuery } from '@mantine/hooks'
import {
  IconBan,
  IconCamera,
  IconFlag,
  IconGauge,
  IconMail,
  IconSearch,
  IconSettings,
  IconShield,
  IconTicket,
  IconUsers,
} from '@tabler/icons-react'
import { Navigate, useNavigate, useSearch } from '@tanstack/react-router'
import { QueryErrorAlert } from '@/components'
import { useMe } from '@/hooks'
import classes from '@/pages/settings/SettingsPage.module.css'
import { AdminEmailPanel } from './AdminEmailPanel'
import { AdminInvitesPanel } from './AdminInvitesPanel'
import { AdminPlatformSettings, AdminQuotasPanel, AdminSearchIndexPanel } from './AdminPlatformSettings'
import { AdminReportsPanel } from './AdminReportsPanel'
import { AdminScreenshotRetryPanel } from './AdminScreenshotRetryPanel'
import { AdminUsersPanel } from './AdminUsersPanel'
import { BannedUsersPanel } from './BannedUsersPanel'

const ADMIN_TABS = [
  'general',
  'users',
  'reports',
  'bans',
  'invites',
  'quotas',
  'email',
  'screenshots',
  'search',
] as const
type AdminTab = (typeof ADMIN_TABS)[number]

function isAdminTab(v: unknown): v is AdminTab {
  return typeof v === 'string' && (ADMIN_TABS as readonly string[]).includes(v)
}

export function AdminPage(): ReactElement | null {
  const me = useMe()
  const nav = useNavigate()
  const search = useSearch({ strict: false }) as { tab?: string }
  const tab: AdminTab = isAdminTab(search.tab) ? search.tab : 'general'
  // Sections run down the side on wide screens, and as a scrolling row on phones.
  const wide = useMediaQuery('(min-width: 62em)', true)

  if (me.isError) return <QueryErrorAlert error={me.error} onRetry={() => void me.refetch()} />
  if (!me.data) return null
  if (!me.data.user.is_admin) return <Navigate to="/" replace />

  function setTab(next: string | null): void {
    if (!isAdminTab(next)) return
    void nav({ to: '/admin', search: next === 'general' ? {} : { tab: next } })
  }

  return (
    <Stack gap="xl">
      <Group gap="md">
        <ThemeIcon size={40} radius="md" variant="light" color="violet">
          <IconShield size={22} stroke={1.5} />
        </ThemeIcon>
        <Title order={2}>Admin</Title>
      </Group>

      <Tabs
        value={tab}
        onChange={setTab}
        variant="pills"
        orientation={wide ? 'vertical' : 'horizontal'}
        keepMounted={false}
        classNames={{ list: classes.list, panel: classes.panel, tab: classes.tab, tabLabel: classes.tabLabel }}
      >
        <Tabs.List aria-label="Admin sections">
          <Tabs.Tab value="general" leftSection={<IconSettings size={16} stroke={1.5} />}>
            General
          </Tabs.Tab>
          <Tabs.Tab value="users" leftSection={<IconUsers size={16} stroke={1.5} />}>
            Users
          </Tabs.Tab>
          <Tabs.Tab value="reports" leftSection={<IconFlag size={16} stroke={1.5} />}>
            Reports
          </Tabs.Tab>
          <Tabs.Tab value="bans" leftSection={<IconBan size={16} stroke={1.5} />}>
            Bans
          </Tabs.Tab>
          <Tabs.Tab value="invites" leftSection={<IconTicket size={16} stroke={1.5} />}>
            Invites
          </Tabs.Tab>
          <Tabs.Tab value="quotas" leftSection={<IconGauge size={16} stroke={1.5} />}>
            Quotas
          </Tabs.Tab>
          <Tabs.Tab value="email" leftSection={<IconMail size={16} stroke={1.5} />}>
            Email
          </Tabs.Tab>
          <Tabs.Tab value="screenshots" leftSection={<IconCamera size={16} stroke={1.5} />}>
            Screenshots
          </Tabs.Tab>
          <Tabs.Tab value="search" leftSection={<IconSearch size={16} stroke={1.5} />}>
            Search index
          </Tabs.Tab>
        </Tabs.List>

        <Tabs.Panel value="general">
          <Stack gap="lg" maw={1100}>
            <AdminPlatformSettings />
          </Stack>
        </Tabs.Panel>
        <Tabs.Panel value="users">
          <AdminUsersPanel />
        </Tabs.Panel>
        <Tabs.Panel value="reports">
          <AdminReportsPanel />
        </Tabs.Panel>
        <Tabs.Panel value="bans">
          <BannedUsersPanel />
        </Tabs.Panel>
        <Tabs.Panel value="invites">
          <Stack gap="lg" maw={1100}>
            <AdminInvitesPanel />
          </Stack>
        </Tabs.Panel>
        <Tabs.Panel value="quotas">
          <Stack gap="lg" maw={1100}>
            <AdminQuotasPanel />
          </Stack>
        </Tabs.Panel>
        <Tabs.Panel value="email">
          <Stack gap="lg" maw={1100}>
            <AdminEmailPanel />
          </Stack>
        </Tabs.Panel>
        <Tabs.Panel value="screenshots">
          <Stack gap="lg" maw={1100}>
            <AdminScreenshotRetryPanel />
          </Stack>
        </Tabs.Panel>
        <Tabs.Panel value="search">
          <Stack gap="lg" maw={1100}>
            <AdminSearchIndexPanel />
          </Stack>
        </Tabs.Panel>
      </Tabs>
    </Stack>
  )
}
