import { useEffect, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate, useSearch } from '@tanstack/react-router'
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
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  IconArrowsExchange,
  IconFolder,
  IconLock,
  IconSettings,
} from '@tabler/icons-react'
import { api, type UserSettings } from '../api'
import { CategoryManager } from '../components/CategoryManager'
import { ImportExportPanel } from '../components/ImportExportPanel'
import { VaultPanel } from '../components/VaultPanel'
import { applyTheme } from '../hooks/usePrefs'
import { useMantineColorScheme } from '@mantine/core'
import { useUi } from '../store'

const SETTINGS_TABS = ['general', 'categories', 'import', 'vault'] as const
type SettingsTab = (typeof SETTINGS_TABS)[number]

function isSettingsTab(v: unknown): v is SettingsTab {
  return typeof v === 'string' && (SETTINGS_TABS as readonly string[]).includes(v)
}

function SettingsSkeleton() {
  return (
    <Stack gap="md" maw={560}>
      <Skeleton height={48} />
      <Skeleton height={64} />
      <Skeleton height={64} />
      <Skeleton height={48} />
    </Stack>
  )
}

export function SettingsPage() {
  const qc = useQueryClient()
  const nav = useNavigate()
  const search = useSearch({ strict: false }) as { tab?: string }
  const tab: SettingsTab = isSettingsTab(search.tab) ? search.tab : 'general'

  const setShowHidden = useUi((s) => s.setShowHidden)
  const { setColorScheme } = useMantineColorScheme()
  const me = useQuery({ queryKey: ['me'], queryFn: api.me })
  const settings = me.data?.settings
  const [autoLock, setAutoLock] = useState(0)

  useEffect(() => {
    if (settings) setAutoLock(settings.vault_auto_lock_minutes)
  }, [settings?.vault_auto_lock_minutes])

  const save = useMutation({
    mutationFn: (patch: Partial<UserSettings>) => api.updateSettings(patch),
    onSuccess: (res) => {
      qc.setQueryData(['me'], (old: typeof me.data) =>
        old ? { ...old, settings: res.settings } : old,
      )
      if (useUi.getState().showHidden !== res.settings.show_hidden) {
        setShowHidden(res.settings.show_hidden)
      }
      applyTheme(res.settings.theme, setColorScheme)
      notifications.show({ message: 'Settings saved' })
    },
    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
  })

  function patch(p: Partial<UserSettings>) {
    if (save.isPending) return
    save.mutate(p)
  }

  function setTab(next: string | null) {
    if (!isSettingsTab(next)) return
    nav({
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
