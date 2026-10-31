import { useState, type ReactElement } from 'react'
import {
  Checkbox,
  Divider,
  Group,
  NumberInput,
  Paper,
  Select,
  Skeleton,
  Stack,
  Switch,
  Tabs,
  Text,
  ThemeIcon,
  Title,
  useMantineColorScheme,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconArrowsExchange, IconFolder, IconLock, IconSettings, IconShield } from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { CategoryManager, ImportExportPanel, QueryErrorAlert, VaultPanel } from '@/components'
import {
  applyTheme,
  useMe,
  usePlatformSettings,
  useUpdatePlatformSettings,
  useUpdateSettings,
} from '@/hooks'
import type { UserSettings } from '@/schemas'

const SETTINGS_TABS = ['general', 'categories', 'import', 'vault', 'admin'] as const
type SettingsTab = (typeof SETTINGS_TABS)[number]

const DEFAULT_AUTO_LOCK_MINUTES = 15

function isSettingsTab(v: unknown): v is SettingsTab {
  return typeof v === 'string' && (SETTINGS_TABS as readonly string[]).includes(v)
}

function SettingsSkeleton(): ReactElement {
  return (
    <Stack gap="md" maw={560}>
      <Skeleton height={48} />
      <Skeleton height={64} />
      <Skeleton height={64} />
      <Skeleton height={48} />
    </Stack>
  )
}

function VaultAutoLockSettings({
  minutes,
  saving,
  onChange,
}: {
  minutes: number
  saving: boolean
  onChange: (minutes: number) => void
}): ReactElement {
  const enabled = minutes > 0
  const [draftMinutes, setDraftMinutes] = useState(minutes > 0 ? minutes : DEFAULT_AUTO_LOCK_MINUTES)

  return (
    <Paper p="xl" withBorder maw={480}>
      <Stack gap="md">
        <Checkbox
          label="Auto-lock vault after inactivity"
          description="When off, vault stays unlocked until you lock it or log out."
          checked={enabled}
          disabled={saving}
          onChange={(e) => {
            if (e.currentTarget.checked) {
              const next = draftMinutes > 0 ? draftMinutes : DEFAULT_AUTO_LOCK_MINUTES
              setDraftMinutes(next)
              onChange(next)
            } else {
              onChange(0)
            }
          }}
        />
        <NumberInput
          label="Timeout (minutes)"
          description="Locks the vault after this many minutes with no activity."
          min={1}
          max={1440}
          disabled={!enabled || saving}
          value={draftMinutes}
          onChange={(v) => setDraftMinutes(typeof v === 'number' ? v : DEFAULT_AUTO_LOCK_MINUTES)}
          onBlur={() => {
            if (!enabled) return
            const next = draftMinutes >= 1 ? Math.min(draftMinutes, 1440) : DEFAULT_AUTO_LOCK_MINUTES
            setDraftMinutes(next)
            if (next !== minutes) onChange(next)
          }}
        />
      </Stack>
    </Paper>
  )
}

function AdminPlatformSettings(): ReactElement {
  const platform = usePlatformSettings(true)
  const save = useUpdatePlatformSettings()

  if (platform.isError) {
    return <QueryErrorAlert error={platform.error} onRetry={() => void platform.refetch()} title="Platform settings failed" />
  }

  if (!platform.data) {
    return <SettingsSkeleton />
  }

  const open = platform.data.settings.registration_open

  return (
    <Paper p="xl" withBorder maw={560}>
      <Stack gap="lg">
        <Switch
          label="Open registration"
          description="When off, only existing users can sign in. First-install bootstrap still works."
          checked={open}
          disabled={save.isPending}
          onChange={(e) => {
            const registration_open = e.currentTarget.checked
            save.mutate(
              { registration_open },
              {
                onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                onError: (err) => notifications.show({ color: 'red', message: String(err) }),
              }
            )
          }}
        />
        {save.isPending ? (
          <Text size="xs" c="dimmed">
            Saving…
          </Text>
        ) : null}
      </Stack>
    </Paper>
  )
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

  function patch(p: Partial<UserSettings>): void {
    if (save.isPending) return
    save.mutate(p, {
      onSuccess: (res) => {
        applyTheme(res.settings.theme, setColorScheme)
        notifications.show({ message: 'Settings saved' })
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
        <Title order={2}>Settings</Title>
      </Group>

      {me.isError ? <QueryErrorAlert error={me.error} onRetry={() => void me.refetch()} /> : null}

      <Tabs value={tab} onChange={setTab}>
        <Tabs.List>
          <Tabs.Tab value="general" leftSection={<IconSettings size={16} stroke={1.5} />}>
            General
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
          {isAdmin ? (
            <Tabs.Tab value="admin" leftSection={<IconShield size={16} stroke={1.5} />}>
              Admin
            </Tabs.Tab>
          ) : null}
        </Tabs.List>

        <Tabs.Panel value="general" pt="xl">
          {!settings ? (
            <SettingsSkeleton />
          ) : (
            <Paper p="xl" withBorder maw={560}>
              <Stack gap="lg">
                <Switch
                  label="Show hidden categories"
                  description="Reveal categories marked hidden in filters and settings"
                  checked={settings.show_hidden}
                  onChange={(e) => patch({ show_hidden: e.currentTarget.checked })}
                />

                <Divider />

                <Select
                  label="Vault in search"
                  description="When vault hits appear in library search"
                  value={settings.vault_search_mode}
                  onChange={(v) =>
                    v &&
                    patch({
                      vault_search_mode: v as UserSettings['vault_search_mode'],
                    })
                  }
                  data={[
                    { value: 'never', label: 'Never' },
                    { value: 'when_unlocked', label: 'Only when unlocked' },
                    { value: 'always', label: 'Always (prompt unlock if locked)' },
                  ]}
                />

                <Select
                  label="Theme"
                  value={settings.theme}
                  onChange={(v) => v && patch({ theme: v as UserSettings['theme'] })}
                  data={[
                    { value: 'system', label: 'System' },
                    { value: 'light', label: 'Light' },
                    { value: 'dark', label: 'Dark' },
                  ]}
                />

                <Select
                  label="Library view"
                  description="Default browse layout. Library toolbar saves this preference."
                  value={settings.library_view}
                  onChange={(v) => v && patch({ library_view: v as UserSettings['library_view'] })}
                  data={[
                    { value: 'grid', label: 'Grid' },
                    { value: 'explorer', label: 'Explorer (folders)' },
                  ]}
                />

                {save.isPending ? (
                  <Text size="xs" c="dimmed">
                    Saving…
                  </Text>
                ) : null}
              </Stack>
            </Paper>
          )}
        </Tabs.Panel>

        <Tabs.Panel value="categories" pt="xl">
          <Paper p="xl" withBorder maw={640}>
            <CategoryManager />
          </Paper>
        </Tabs.Panel>

        <Tabs.Panel value="import" pt="xl">
          <ImportExportPanel />
        </Tabs.Panel>

        <Tabs.Panel value="vault" pt="xl">
          <Stack gap="lg">
            <VaultPanel />
            {!settings ? (
              <SettingsSkeleton />
            ) : (
              <VaultAutoLockSettings
                minutes={settings.vault_auto_lock_minutes}
                saving={save.isPending}
                onChange={(vault_auto_lock_minutes) => patch({ vault_auto_lock_minutes })}
              />
            )}
            {save.isPending ? (
              <Text size="xs" c="dimmed">
                Saving…
              </Text>
            ) : null}
          </Stack>
        </Tabs.Panel>

        {isAdmin ? (
          <Tabs.Panel value="admin" pt="xl">
            <AdminPlatformSettings />
          </Tabs.Panel>
        ) : null}
      </Tabs>
    </Stack>
  )
}
