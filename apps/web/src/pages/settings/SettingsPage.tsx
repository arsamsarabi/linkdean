import { type ReactElement } from 'react'
import { Group, Paper, Stack, Tabs, Text, ThemeIcon, Title, useMantineColorScheme } from '@mantine/core'
import { useMediaQuery } from '@mantine/hooks'
import { notifications } from '@mantine/notifications'
import {
  IconArrowsExchange,
  IconBell,
  IconFolder,
  IconKey,
  IconLock,
  IconSettings,
  IconShield,
} from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import {
  ApiKeysPanel,
  CategoryManager,
  ImportExportPanel,
  QueryErrorAlert,
  VaultPanel,
  VaultPreferencesCard,
} from '@/components'
import { applyTheme, useMe, useUpdateSettings } from '@/hooks'
import type { UserSettings } from '@/schemas'
import { AdminPlatformSettings } from './AdminPlatformSettings'
import { DeleteAccountCard } from './DeleteAccountCard'
import { GeneralSettingsPanel } from './GeneralSettingsPanel'
import { NotificationEmailsPanel } from './NotificationEmailsPanel'
import classes from './SettingsPage.module.css'
import { SettingsSkeleton } from './SettingsSkeleton'
import { VaultAutoLockSettings } from './VaultAutoLockSettings'

const SETTINGS_TABS = ['general', 'notifications', 'categories', 'import', 'vault', 'api-keys', 'admin'] as const
type SettingsTab = (typeof SETTINGS_TABS)[number]

function isSettingsTab(v: unknown): v is SettingsTab {
  return typeof v === 'string' && (SETTINGS_TABS as readonly string[]).includes(v)
}

export function SettingsPage(): ReactElement {
  const nav = useNavigate()
  const search = useSearch({ strict: false }) as { tab?: string }
  const { setColorScheme } = useMantineColorScheme()
  const me = useMe()
  const isAdmin = !!me.data?.user.is_admin
  const requested: SettingsTab = isSettingsTab(search.tab) ? search.tab : 'general'
  const tab: SettingsTab = requested === 'admin' && !isAdmin ? 'general' : requested
  const settings = me.data?.settings
  const save = useUpdateSettings()
  // Sections run down the side on wide screens, and as a scrolling row on phones.
  const wide = useMediaQuery('(min-width: 62em)', true)

  function patch(p: Partial<UserSettings>): void {
    if (save.isPending) return
    save.mutate(p, {
      onSuccess: (res) => {
        applyTheme(res.settings.theme, setColorScheme)
        notifications.show({ message: 'Config options saved' })
      },
      onError: (e) => notifications.show({ color: 'red', message: String(e) }),
    })
  }

  function setTab(next: string | null): void {
    if (!isSettingsTab(next)) return
    if (next === 'admin' && !isAdmin) return
    void nav({
      to: '/settings',
      search: next === 'general' ? {} : { tab: next },
    })
  }

  return (
    <Stack gap="xl">
      <Group gap="md">
        <ThemeIcon size={40} radius="md" variant="light" color="violet">
          <IconSettings size={22} stroke={1.5} />
        </ThemeIcon>
        <Title order={2}>Config options</Title>
      </Group>

      {me.isError ? <QueryErrorAlert error={me.error} onRetry={() => void me.refetch()} /> : null}

      <Tabs
        value={tab}
        onChange={setTab}
        variant="pills"
        orientation={wide ? 'vertical' : 'horizontal'}
        classNames={{ list: classes.list, panel: classes.panel, tab: classes.tab, tabLabel: classes.tabLabel }}
      >
        <Tabs.List aria-label="Settings sections">
          <Tabs.Tab value="general" leftSection={<IconSettings size={16} stroke={1.5} />}>
            General
          </Tabs.Tab>
          <Tabs.Tab value="notifications" leftSection={<IconBell size={16} stroke={1.5} />}>
            Notifications
          </Tabs.Tab>
          <Tabs.Tab value="categories" leftSection={<IconFolder size={16} stroke={1.5} />}>
            Categories
          </Tabs.Tab>
          <Tabs.Tab value="import" leftSection={<IconArrowsExchange size={16} stroke={1.5} />}>
            Import / Export
          </Tabs.Tab>
          <Tabs.Tab value="vault" leftSection={<IconLock size={16} stroke={1.5} />}>
            Vault
          </Tabs.Tab>
          <Tabs.Tab value="api-keys" leftSection={<IconKey size={16} stroke={1.5} />}>
            API keys
          </Tabs.Tab>
          {isAdmin ? (
            <Tabs.Tab value="admin" leftSection={<IconShield size={16} stroke={1.5} />}>
              Admin
            </Tabs.Tab>
          ) : null}
        </Tabs.List>

        <Tabs.Panel value="general">
          <Stack gap="lg" maw={1100}>
            <GeneralSettingsPanel settings={settings} saving={save.isPending} onPatch={patch} />
            <DeleteAccountCard user={me.data?.user} />
          </Stack>
        </Tabs.Panel>

        <Tabs.Panel value="notifications">
          <NotificationEmailsPanel />
        </Tabs.Panel>

        <Tabs.Panel value="categories">
          <Paper p="xl" withBorder>
            <CategoryManager />
          </Paper>
        </Tabs.Panel>

        <Tabs.Panel value="import">
          <ImportExportPanel />
        </Tabs.Panel>

        <Tabs.Panel value="vault">
          <VaultPanel
            settings={
              <>
                {!settings ? (
                  <SettingsSkeleton />
                ) : (
                  <>
                    <VaultPreferencesCard settings={settings} onPatch={patch} />
                    <VaultAutoLockSettings
                      minutes={settings.vault_auto_lock_minutes}
                      saving={save.isPending}
                      onChange={(vault_auto_lock_minutes) => patch({ vault_auto_lock_minutes })}
                    />
                  </>
                )}
                {save.isPending ? (
                  <Text size="xs" c="dimmed">
                    Saving…
                  </Text>
                ) : null}
              </>
            }
          />
        </Tabs.Panel>

        <Tabs.Panel value="api-keys">
          <ApiKeysPanel />
        </Tabs.Panel>

        {isAdmin ? (
          <Tabs.Panel value="admin">
            <AdminPlatformSettings />
          </Tabs.Panel>
        ) : null}
      </Tabs>
    </Stack>
  )
}
