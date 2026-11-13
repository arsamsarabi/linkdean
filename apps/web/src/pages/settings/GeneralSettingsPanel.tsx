import { useState, type ReactElement, type ReactNode } from 'react'
import { NumberInput, Paper, Select, SimpleGrid, Stack, Switch, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useDashboards, usePatchDashboard } from '@/hooks'
import type { UserSettings } from '@/schemas'
import { QuotaUsagePanel } from './QuotaUsagePanel'
import { SettingsSkeleton } from './SettingsSkeleton'

type Props = {
  settings: UserSettings | undefined
  saving: boolean
  onPatch: (patch: Partial<UserSettings>) => void
}

function SettingsCard({
  title,
  description,
  children,
}: {
  title: string
  description: string
  children: ReactNode
}): ReactElement {
  return (
    <Paper p="lg" withBorder>
      <Stack gap="lg">
        <Stack gap={2}>
          <Title order={4}>{title}</Title>
          <Text size="sm" c="dimmed">
            {description}
          </Text>
        </Stack>
        {children}
      </Stack>
    </Paper>
  )
}

/** Saves on blur, so typing 365 never stores 3 on the way. Keyed on `days` to reset on save. */
function RetentionInput({ days, onSave }: { days: number; onSave: (days: number) => void }): ReactElement {
  const [draft, setDraft] = useState<string | number>(days)
  const commit = (): void => {
    const n = Math.floor(Number(draft))
    if (Number.isFinite(n) && n >= 1 && n <= 3650 && n !== days) onSave(n)
    else setDraft(days)
  }
  return (
    <NumberInput
      label="Keep deleted items for (days)"
      description="Items in Deleted items are purged after this many days"
      min={1}
      max={3650}
      value={draft}
      onChange={setDraft}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === 'Enter') commit()
      }}
    />
  )
}

export function GeneralSettingsPanel({ settings, saving, onPatch }: Props): ReactElement {
  const dashboards = useDashboards()
  const pinDashboard = usePatchDashboard()

  if (!settings) {
    return <SettingsSkeleton />
  }

  return (
    <Stack gap="lg" maw={1100}>
      <SimpleGrid cols={{ base: 1, md: 2 }} spacing="lg">
        <SettingsCard title="Appearance" description="How Linkdean looks, and where it opens.">
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
        </SettingsCard>

        <SettingsCard title="Keyboard" description="Shortcuts work the same either way.">
          <Switch
            label="Show keyboard hints"
            description="Key numbers on dashboard bookmarks, key hint lines, and shortcut keys such as ⌘K and /. Turn off for a cleaner look."
            checked={settings.show_keyboard_hints}
            onChange={(e) => onPatch({ show_keyboard_hints: e.currentTarget.checked })}
          />
        </SettingsCard>

        <SettingsCard title="Library" description="How bookmarks are browsed and searched.">
          <Select
            label="Library view"
            description="Default browse layout. Library toolbar saves this preference."
            value={settings.library_view}
            onChange={(v) => v && onPatch({ library_view: v as UserSettings['library_view'] })}
            data={[
              { value: 'grid', label: 'Grid' },
              { value: 'list', label: 'List (compact)' },
              { value: 'explorer', label: 'Explorer (folders)' },
            ]}
          />
          <Switch
            label="Show hidden categories"
            description="Reveal categories marked hidden in filters and Config options"
            checked={settings.show_hidden}
            onChange={(e) => onPatch({ show_hidden: e.currentTarget.checked })}
          />
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
        </SettingsCard>

        <SettingsCard title="Deleted items" description="What happens to bookmarks and folders you delete.">
          <Switch
            label="Show pending delete in library"
            description="Ghost soft-deleted bookmarks and folders in library and dashboards while they wait in Deleted items"
            checked={settings.show_deleted_in_library}
            onChange={(e) => onPatch({ show_deleted_in_library: e.currentTarget.checked })}
          />
          <RetentionInput
            key={settings.trash_retention_days}
            days={settings.trash_retention_days}
            onSave={(trash_retention_days) => onPatch({ trash_retention_days })}
          />
        </SettingsCard>
      </SimpleGrid>

      <QuotaUsagePanel />

      {saving || pinDashboard.isPending ? (
        <Text size="xs" c="dimmed" aria-live="polite">
          Saving…
        </Text>
      ) : null}
    </Stack>
  )
}
