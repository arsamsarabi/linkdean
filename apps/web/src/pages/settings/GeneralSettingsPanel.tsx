import { type ReactElement } from 'react'
import { Divider, NumberInput, Paper, Select, Stack, Switch, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useDashboards, usePatchDashboard } from '@/hooks'
import type { UserSettings } from '@/schemas'
import { SettingsSkeleton } from './SettingsSkeleton'

type Props = {
  settings: UserSettings | undefined
  saving: boolean
  onPatch: (patch: Partial<UserSettings>) => void
}

export function GeneralSettingsPanel({ settings, saving, onPatch }: Props): ReactElement {
  const dashboards = useDashboards()
  const pinDashboard = usePatchDashboard()

  if (!settings) {
    return <SettingsSkeleton />
  }

  return (
    <Paper p="xl" withBorder maw={560}>
      <Stack gap="lg">
        <Switch
          label="Show hidden categories"
          description="Reveal categories marked hidden in filters and Config options"
          checked={settings.show_hidden}
          onChange={(e) => onPatch({ show_hidden: e.currentTarget.checked })}
        />

        <Switch
          label="Show pending delete in library"
          description="Ghost soft-deleted bookmarks and folders in library and dashboards while they wait in Deleted items"
          checked={settings.show_deleted_in_library}
          onChange={(e) => onPatch({ show_deleted_in_library: e.currentTarget.checked })}
        />

        <NumberInput
          label="Deleted retention (days)"
          description="Soft-deleted items purge automatically after this many days"
          min={1}
          max={3650}
          value={settings.trash_retention_days}
          onChange={(v) => {
            const n = typeof v === 'number' ? v : Number(v)
            if (Number.isFinite(n) && n > 0) onPatch({ trash_retention_days: Math.floor(n) })
          }}
        />

        <Divider />

        {settings.show_vault_in_library ? (
          <Select
            label="Vault in search"
            description="When vault hits appear in library search"
            value={settings.vault_search_mode}
            onChange={(v) =>
              v &&
              onPatch({
                vault_search_mode: v as UserSettings['vault_search_mode'],
              })
            }
            data={[
              { value: 'never', label: 'Never' },
              { value: 'when_unlocked', label: 'Only when unlocked' },
              { value: 'always', label: 'Always (prompt unlock if locked)' },
            ]}
          />
        ) : null}

        <Select
          label="Theme"
          value={settings.theme}
          onChange={(v) => v && onPatch({ theme: v as UserSettings['theme'] })}
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
          onChange={(v) => v && onPatch({ library_view: v as UserSettings['library_view'] })}
          data={[
            { value: 'grid', label: 'Grid' },
            { value: 'explorer', label: 'Explorer (folders)' },
          ]}
        />

        <Select
          label="Default landing"
          description="Where you land after sign-in."
          value={settings.default_landing}
          onChange={(v) => v && onPatch({ default_landing: v as UserSettings['default_landing'] })}
          data={[
            { value: '/', label: 'Dashboard (home)' },
            { value: '/library', label: 'Library' },
          ]}
        />

        <Select
          label="Default dashboard"
          description="Which board opens on the Dashboard page (pin)."
          value={dashboards.data?.find((d) => d.is_default)?.id || null}
          disabled={!dashboards.data || pinDashboard.isPending}
          onChange={(id) => {
            if (!id) return
            pinDashboard.mutate(
              { id, body: { is_default: true } },
              {
                onSuccess: () => notifications.show({ message: 'Default dashboard updated' }),
                onError: (err) => notifications.show({ color: 'red', message: String(err) }),
              }
            )
          }}
          data={(dashboards.data || [])
            .filter((d) => d.access === 'owner')
            .map((d) => ({
              value: d.id,
              label: d.title,
            }))}
        />

        {saving || pinDashboard.isPending ? (
          <Text size="xs" c="dimmed">
            Saving…
          </Text>
        ) : null}
      </Stack>
    </Paper>
  )
}
