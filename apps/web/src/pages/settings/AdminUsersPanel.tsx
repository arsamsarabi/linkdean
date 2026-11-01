import { useMemo, useState, type ReactElement } from 'react'
import { Divider, Group, Pagination, Paper, Stack, Switch, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconSearch } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components'
import { useAdminUsers, useUpdateAdminUser } from '@/hooks'
import type { AdminUserSummary } from '@/schemas'
import { SettingsSkeleton } from './SettingsSkeleton'

const ADMIN_USERS_PAGE_SIZE = 10

function matchesAdminUser(u: AdminUserSummary, q: string): boolean {
  const needle = q.trim().toLowerCase()
  if (!needle) return true
  return u.email.toLowerCase().includes(needle) || (u.display_name?.toLowerCase().includes(needle) ?? false)
}

export function AdminUsersPanel(): ReactElement {
  const users = useAdminUsers(true)
  const patchUser = useUpdateAdminUser()
  const [userQuery, setUserQuery] = useState('')
  const [page, setPage] = useState(1)
  const allUsers = users.data?.users ?? []
  const adminCount = allUsers.filter((u) => u.is_admin).length

  const filteredUsers = useMemo(() => allUsers.filter((u) => matchesAdminUser(u, userQuery)), [allUsers, userQuery])
  const totalPages = Math.max(1, Math.ceil(filteredUsers.length / ADMIN_USERS_PAGE_SIZE))
  const safePage = Math.min(page, totalPages)
  const pageUsers = useMemo(() => {
    const start = (safePage - 1) * ADMIN_USERS_PAGE_SIZE
    return filteredUsers.slice(start, start + ADMIN_USERS_PAGE_SIZE)
  }, [filteredUsers, safePage])

  return (
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
        {users.isError ? <QueryErrorAlert error={users.error} onRetry={() => void users.refetch()} /> : null}
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
              {filteredUsers.length > ADMIN_USERS_PAGE_SIZE ? ` · page ${safePage} of ${totalPages}` : ''}
            </Text>
            {totalPages > 1 ? <Pagination value={safePage} onChange={setPage} total={totalPages} size="sm" /> : null}
          </Group>
        ) : null}
      </Stack>
    </Paper>
  )
}
