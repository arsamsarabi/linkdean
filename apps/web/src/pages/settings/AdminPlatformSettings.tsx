import { useState, type ReactElement } from 'react'
import { Button, NumberInput, Paper, Stack, Switch, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconRefresh } from '@tabler/icons-react'
import { QueryErrorAlert, TwoColumnLayout } from '@/components'
import { useBootstrap, usePlatformSettings, useReindexSearch, useUpdatePlatformSettings } from '@/hooks'
import { AdminEmailPanel } from './AdminEmailPanel'
import { AdminInvitesPanel } from './AdminInvitesPanel'
import { AdminReportsPanel } from './AdminReportsPanel'
import { AdminUsersPanel } from './AdminUsersPanel'
import { BannedUsersPanel } from './BannedUsersPanel'
import { SettingsSkeleton } from './SettingsSkeleton'

function QuotaNumberInput({
  label,
  description,
  value,
  disabled,
  onSave,
}: {
  label: string
  description: string
  value: number | null | undefined
  disabled: boolean
  onSave: (v: number | null) => void
}): ReactElement {
  const [draft, setDraft] = useState<string | number>(value ?? '')
  return (
    <NumberInput
      key={value ?? 'unlimited'}
      label={label}
      description={description}
      min={1}
      value={draft}
      disabled={disabled}
      placeholder="Unlimited"
      onChange={setDraft}
      onBlur={() => {
        if (draft === '' || draft === null || draft === undefined) {
          if (value != null) onSave(null)
          setDraft('')
          return
        }
        const n = Math.floor(Number(draft))
        if (Number.isFinite(n) && n >= 1 && n !== value) onSave(n)
        else setDraft(value ?? '')
      }}
    />
  )
}

export function AdminPlatformSettings(): ReactElement {
  const platform = usePlatformSettings(true)
  const save = useUpdatePlatformSettings()
  const reindex = useReindexSearch()
  const mailEnabled = useBootstrap().data?.mail_enabled ?? false

  if (platform.isError) {
    return (
      <QueryErrorAlert
        error={platform.error}
        onRetry={() => void platform.refetch()}
        title="Platform settings failed"
      />
    )
  }

  if (!platform.data) {
    return <SettingsSkeleton />
  }

  const open = platform.data.settings.registration_open
  const apiKeysOn = platform.data.settings.api_keys_enabled
  const requireVerified = platform.data.settings.require_verified_email
  const requireForced = platform.data.require_verified_email_forced

  return (
    <TwoColumnLayout
      left={
        <>
          <Paper p="xl" withBorder>
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
              <Switch
                label="API keys enabled"
                description="When off, no user can create or authenticate with API keys."
                checked={apiKeysOn}
                disabled={save.isPending}
                onChange={(e) => {
                  const api_keys_enabled = e.currentTarget.checked
                  save.mutate(
                    { api_keys_enabled },
                    {
                      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }}
              />
              <Switch
                label="Require verified email"
                description={
                  requireForced
                    ? 'Set by the host (REQUIRE_VERIFIED_EMAIL). Unverified accounts can only verify or sign out.'
                    : 'Unverified accounts can only verify or sign out. Needs outbound email.'
                }
                checked={requireVerified}
                // Host force locks the toggle. Turning it off stays possible with mail off otherwise.
                disabled={save.isPending || requireForced || (!mailEnabled && !requireVerified)}
                onChange={(e) => {
                  const require_verified_email = e.currentTarget.checked
                  save.mutate(
                    { require_verified_email },
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
          <Paper p="xl" withBorder>
            <Stack gap="md">
              <Text fw={500}>Per-user quotas</Text>
              <Text size="sm" c="dimmed">
                Leave blank for unlimited. Applies to every account. Clear a field and blur to remove a limit.
              </Text>
              <QuotaNumberInput
                label="Max bookmarks"
                description="Owned bookmarks (not deleted)"
                value={platform.data.settings.max_bookmarks}
                disabled={save.isPending}
                onSave={(max_bookmarks) =>
                  save.mutate(
                    { max_bookmarks },
                    {
                      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }
              />
              <QuotaNumberInput
                label="Max stored image bytes"
                description="Avatar + preview/favicon/OG files"
                value={platform.data.settings.max_stored_image_bytes}
                disabled={save.isPending}
                onSave={(max_stored_image_bytes) =>
                  save.mutate(
                    { max_stored_image_bytes },
                    {
                      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }
              />
              <QuotaNumberInput
                label="Max enrich actions per day (UTC)"
                description="Page, favicon, OG, and screenshot fetches"
                value={platform.data.settings.max_enrich_actions_per_day}
                disabled={save.isPending}
                onSave={(max_enrich_actions_per_day) =>
                  save.mutate(
                    { max_enrich_actions_per_day },
                    {
                      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }
              />
              <QuotaNumberInput
                label="Max import bytes"
                description="Clamped to 10 MiB hard cap"
                value={platform.data.settings.max_import_bytes}
                disabled={save.isPending}
                onSave={(max_import_bytes) =>
                  save.mutate(
                    { max_import_bytes },
                    {
                      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }
              />
              <QuotaNumberInput
                label="Max upload image bytes"
                description="Clamped to avatar/preview hard caps"
                value={platform.data.settings.max_upload_image_bytes}
                disabled={save.isPending}
                onSave={(max_upload_image_bytes) =>
                  save.mutate(
                    { max_upload_image_bytes },
                    {
                      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }
              />
            </Stack>
          </Paper>
          <Paper p="xl" withBorder>
            <Stack gap="md">
              <Text fw={500}>Search index</Text>
              <Text size="sm" c="dimmed">
                Rebuild Meilisearch from Postgres (non-vault bookmarks). Clears stale docs first.
              </Text>
              <Button
                leftSection={<IconRefresh size={16} stroke={1.5} />}
                loading={reindex.isPending}
                onClick={() => {
                  reindex.mutate(undefined, {
                    onSuccess: (res) =>
                      notifications.show({
                        message: `Reindex done · ${res.upserted} upserted${res.failed ? ` · ${res.failed} failed` : ''}`,
                      }),
                    onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                  })
                }}
              >
                Reindex search
              </Button>
            </Stack>
          </Paper>
          <AdminInvitesPanel />
          <AdminEmailPanel />
        </>
      }
      right={
        <>
          <AdminReportsPanel />
          <BannedUsersPanel />
          <AdminUsersPanel />
        </>
      }
    />
  )
}
