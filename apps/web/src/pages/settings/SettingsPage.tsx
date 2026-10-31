import { useEffect, useState, type ReactElement } from 'react'
import {
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
import { IconArrowsExchange, IconFolder, IconLock, IconSettings } from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { CategoryManager, ImportExportPanel, QueryErrorAlert, VaultPanel } from '@/components'
import { applyTheme, useMe, useUpdateSettings } from '@/hooks'
import type { UserSettings } from '@/schemas'

const SETTINGS_TABS = ['general', 'categories', 'import', 'vault'] as const
type SettingsTab = (typeof SETTINGS_TABS)[number]

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

export function SettingsPage(): ReactElement {
  const nav = useNavigate()
  const search = useSearch({ strict: false }) as { tab?: string }
  const tab: SettingsTab = isSettingsTab(search.tab) ? search.tab : 'general'

  const { setColorScheme } = useMantineColorScheme()
  const me = useMe()
  const settings = me.data?.settings
  const [autoLock, setAutoLock] = useState(0)
  const save = useUpdateSettings()

  useEffect(() => {
    if (settings) setAutoLock(settings.vault_auto_lock_minutes)
  }, [settings, settings?.vault_auto_lock_minutes])

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

                <NumberInput
                  label="Vault auto-lock (minutes)"
                  description="0 = never. Locks vault after idle."
                  min={0}
                  max={1440}
                  value={autoLock}
                  onChange={(v) => setAutoLock(typeof v === 'number' ? v : 0)}
                  onBlur={() => {
                    if (autoLock === settings.vault_auto_lock_minutes) return
                    patch({ vault_auto_lock_minutes: autoLock })
                  }}
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
          <VaultPanel />
        </Tabs.Panel>
      </Tabs>
    </Stack>
  )
}
