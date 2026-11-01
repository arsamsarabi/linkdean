import { useMemo, useState, type ReactElement } from 'react'
import {
  Button,
  Checkbox,
  Divider,
  Group,
  NumberInput,
  Pagination,
  Paper,
  Select,
  Skeleton,
  Stack,
  Switch,
  Tabs,
  Text,
  TextInput,
  ThemeIcon,
  Title,
  useMantineColorScheme,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import {
  IconArrowsExchange,
  IconCopy,
  IconFolder,
  IconKey,
  IconLock,
  IconRefresh,
  IconSearch,
  IconSettings,
  IconShield,
  IconTicket,
} from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import {
  ApiKeysPanel,
  CategoryManager,
  ImportExportPanel,
  QueryErrorAlert,
  VaultPanel,
} from '@/components'
import {
  applyTheme,
  useAdminUsers,
  useCreateRegistrationInvite,
  useDashboards,
  useMe,
  usePatchDashboard,
  usePlatformSettings,
  useRegistrationInvites,
  useReindexSearch,
  useRevokeRegistrationInvite,
  useUpdateAdminUser,
  useUpdatePlatformSettings,
  useUpdateSettings,
} from '@/hooks'
import type { AdminUserSummary, UserSettings } from '@/schemas'

const SETTINGS_TABS = ['general', 'categories', 'import', 'vault', 'api-keys', 'admin'] as const
type SettingsTab = (typeof SETTINGS_TABS)[number]

const DEFAULT_AUTO_LOCK_MINUTES = 15
const ADMIN_USERS_PAGE_SIZE = 10

function matchesAdminUser(u: AdminUserSummary, q: string): boolean {
  const needle = q.trim().toLowerCase()
  if (!needle) return true
  return (
    u.email.toLowerCase().includes(needle) ||
    (u.display_name?.toLowerCase().includes(needle) ?? false)
  )
}

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
  const users = useAdminUsers(true)
  const patchUser = useUpdateAdminUser()
  const invites = useRegistrationInvites(true)
  const createInvite = useCreateRegistrationInvite()
  const revokeInvite = useRevokeRegistrationInvite()
  const reindex = useReindexSearch()
  const [userQuery, setUserQuery] = useState('')
  const [page, setPage] = useState(1)
  const [inviteEmail, setInviteEmail] = useState('')
  const [inviteDays, setInviteDays] = useState(7)
  const [freshInviteCode, setFreshInviteCode] = useState<string | null>(null)
  const allUsers = users.data?.users ?? []
  const adminCount = allUsers.filter((u) => u.is_admin).length

  const filteredUsers = useMemo(
    () => allUsers.filter((u) => matchesAdminUser(u, userQuery)),
    [allUsers, userQuery]
  )
  const totalPages = Math.max(1, Math.ceil(filteredUsers.length / ADMIN_USERS_PAGE_SIZE))
  const safePage = Math.min(page, totalPages)
  const pageUsers = useMemo(() => {
    const start = (safePage - 1) * ADMIN_USERS_PAGE_SIZE
    return filteredUsers.slice(start, start + ADMIN_USERS_PAGE_SIZE)
  }, [filteredUsers, safePage])

  if (platform.isError) {
    return <QueryErrorAlert error={platform.error} onRetry={() => void platform.refetch()} title="Platform settings failed" />
  }

  if (!platform.data) {
    return <SettingsSkeleton />
  }

  const open = platform.data.settings.registration_open
  const apiKeysOn = platform.data.settings.api_keys_enabled

  return (
    <Stack gap="lg" maw={640}>
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
          {save.isPending ? (
            <Text size="xs" c="dimmed">
              Saving…
            </Text>
          ) : null}
        </Stack>
      </Paper>

      <Paper p="xl" withBorder>
        <Stack gap="md">
          <div>
            <Text fw={500}>Accounts</Text>
            <Text size="sm" c="dimmed">
              Promote admins and toggle API keys per account. Private user data stays private.
            </Text>
          </div>
          <TextInput
            placeholder="Filter by name or email"
            leftSection={<IconSearch size={16} stroke={1.5} />}
            value={userQuery}
            onChange={(e) => {
              setUserQuery(e.currentTarget.value)
              setPage(1)
            }}
            aria-label="Filter users"
          />
          {users.isError ? (
            <QueryErrorAlert error={users.error} onRetry={() => void users.refetch()} />
          ) : null}
          {users.isLoading ? <SettingsSkeleton /> : null}
          {!users.isLoading && filteredUsers.length === 0 ? (
            <Text size="sm" c="dimmed">
              No users match.
            </Text>
          ) : null}
          {pageUsers.map((u, i) => (
            <Stack key={u.id} gap="xs">
              <div>
                <Text size="sm" fw={500}>
                  {u.display_name?.trim() || u.email}
                </Text>
                <Text size="xs" c="dimmed">
                  {u.email}
                </Text>
              </div>
              <Switch
                label="Admin"
                description="Can change platform settings and manage accounts."
                checked={u.is_admin}
                disabled={patchUser.isPending || (u.is_admin && adminCount <= 1)}
                onChange={(e) => {
                  const is_admin = e.currentTarget.checked
                  patchUser.mutate(
                    { id: u.id, is_admin },
                    {
                      onSuccess: () => notifications.show({ message: 'User updated' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }}
              />
              <Switch
                label="API keys"
                description="Allow this account to create and use API keys."
                checked={u.api_keys_enabled}
                disabled={patchUser.isPending}
                onChange={(e) => {
                  const api_keys_enabled = e.currentTarget.checked
                  patchUser.mutate(
                    { id: u.id, api_keys_enabled },
                    {
                      onSuccess: () => notifications.show({ message: 'User updated' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    }
                  )
                }}
              />
              {i < pageUsers.length - 1 ? <Divider /> : null}
            </Stack>
          ))}
          {!users.isLoading && filteredUsers.length > 0 ? (
            <Group justify="space-between" align="center" wrap="wrap" gap="sm">
              <Text size="xs" c="dimmed">
                {filteredUsers.length} user{filteredUsers.length === 1 ? '' : 's'}
                {userQuery.trim() ? ' match' : ''}
                {filteredUsers.length > ADMIN_USERS_PAGE_SIZE
                  ? ` · page ${safePage} of ${totalPages}`
                  : ''}
              </Text>
              {totalPages > 1 ? (
                <Pagination value={safePage} onChange={setPage} total={totalPages} size="sm" />
              ) : null}
            </Group>
          ) : null}
        </Stack>
      </Paper>

      <Paper p="xl" withBorder>
        <Stack gap="md">
          <div>
            <Text fw={500}>Registration invites</Text>
            <Text size="sm" c="dimmed">
              Issue one-time codes when registration is closed. Share the link; email delivery comes
              later with SMTP.
            </Text>
          </div>
          <Group align="flex-end" wrap="wrap" gap="sm">
            <TextInput
              label="Email (optional)"
              placeholder="bind invite to address"
              value={inviteEmail}
              onChange={(e) => setInviteEmail(e.currentTarget.value)}
              style={{ flex: 1, minWidth: 180 }}
            />
            <NumberInput
              label="Expires (days)"
              min={1}
              max={90}
              value={inviteDays}
              onChange={(v) => setInviteDays(typeof v === 'number' ? v : 7)}
              w={120}
            />
            <Button
              leftSection={<IconTicket size={16} stroke={1.5} />}
              loading={createInvite.isPending}
              onClick={() => {
                createInvite.mutate(
                  {
                    email: inviteEmail.trim() || undefined,
                    expires_in_days: inviteDays,
                  },
                  {
                    onSuccess: (res) => {
                      setFreshInviteCode(res.code)
                      setInviteEmail('')
                      notifications.show({ message: 'Invite created — copy the code now' })
                    },
                    onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                  }
                )
              }}
            >
              Create invite
            </Button>
          </Group>
          {freshInviteCode ? (
            <Paper p="md" withBorder radius="md">
              <Stack gap="xs">
                <Text size="sm" fw={500}>
                  New invite (shown once)
                </Text>
                <Text size="xs" ff="monospace" style={{ wordBreak: 'break-all' }}>
                  {freshInviteCode}
                </Text>
                <Group gap="xs">
                  <Button
                    size="compact-sm"
                    variant="light"
                    leftSection={<IconCopy size={14} stroke={1.5} />}
                    onClick={() => {
                      void navigator.clipboard.writeText(freshInviteCode)
                      notifications.show({ message: 'Code copied' })
                    }}
                  >
                    Copy code
                  </Button>
                  <Button
                    size="compact-sm"
                    variant="light"
                    leftSection={<IconCopy size={14} stroke={1.5} />}
                    onClick={() => {
                      const url = `${window.location.origin}/register?invite=${encodeURIComponent(freshInviteCode)}`
                      void navigator.clipboard.writeText(url)
                      notifications.show({ message: 'Invite link copied' })
                    }}
                  >
                    Copy link
                  </Button>
                </Group>
              </Stack>
            </Paper>
          ) : null}
          {invites.isError ? (
            <QueryErrorAlert error={invites.error} onRetry={() => void invites.refetch()} />
          ) : null}
          {invites.isLoading ? <SettingsSkeleton /> : null}
          {(invites.data?.invites ?? []).length === 0 && !invites.isLoading ? (
            <Text size="sm" c="dimmed">
              No invites yet.
            </Text>
          ) : null}
          {(invites.data?.invites ?? []).map((inv) => (
            <Group key={inv.id} justify="space-between" align="flex-start" wrap="wrap" gap="sm">
              <Stack gap={2}>
                <Text size="sm" fw={500} ff="monospace">
                  {inv.code_prefix}…
                </Text>
                <Text size="xs" c="dimmed">
                  {inv.status}
                  {inv.email ? ` · ${inv.email}` : ''}
                  {' · '}expires {new Date(inv.expires_at).toLocaleDateString()}
                </Text>
              </Stack>
              {inv.status === 'pending' ? (
                <Button
                  size="compact-sm"
                  variant="subtle"
                  color="red"
                  loading={revokeInvite.isPending}
                  onClick={() => {
                    revokeInvite.mutate(inv.id, {
                      onSuccess: () => notifications.show({ message: 'Invite revoked' }),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    })
                  }}
                >
                  Revoke
                </Button>
              ) : null}
            </Group>
          ))}
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
    </Stack>
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
  const dashboards = useDashboards()
  const pinDashboard = usePatchDashboard()

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
          <Tabs.Tab value="api-keys" leftSection={<IconKey size={16} stroke={1.5} />}>
            API keys
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
                  description="Reveal categories marked hidden in filters and Config options"
                  checked={settings.show_hidden}
                  onChange={(e) => patch({ show_hidden: e.currentTarget.checked })}
                />

                <Switch
                  label="Show pending delete in library"
                  description="Ghost soft-deleted bookmarks and folders in library and dashboards while they wait in Deleted items"
                  checked={settings.show_deleted_in_library}
                  onChange={(e) => patch({ show_deleted_in_library: e.currentTarget.checked })}
                />

                <NumberInput
                  label="Deleted retention (days)"
                  description="Soft-deleted items purge automatically after this many days"
                  min={1}
                  max={3650}
                  value={settings.trash_retention_days}
                  onChange={(v) => {
                    const n = typeof v === 'number' ? v : Number(v)
                    if (Number.isFinite(n) && n > 0) patch({ trash_retention_days: Math.floor(n) })
                  }}
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

                <Select
                  label="Default landing"
                  description="Where you land after sign-in."
                  value={settings.default_landing}
                  onChange={(v) =>
                    v && patch({ default_landing: v as UserSettings['default_landing'] })
                  }
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

                {save.isPending || pinDashboard.isPending ? (
                  <Text size="xs" c="dimmed">
                    Saving…
                  </Text>
                ) : null}
              </Stack>
            </Paper>
          )}
        </Tabs.Panel>

        <Tabs.Panel value="categories" pt="xl">
          <Paper p="xl" withBorder>
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

        <Tabs.Panel value="api-keys" pt="xl">
          <ApiKeysPanel />
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
