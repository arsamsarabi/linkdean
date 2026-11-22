import { useState, type ReactElement } from 'react'
import { Button, Group, Stack, Table, Tabs, Text, TextInput, ThemeIcon, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconBuilding, IconTrash } from '@tabler/icons-react'
import { useParams } from '@tanstack/react-router'
import { QueryErrorAlert } from '@/components'
import {
  useArchiveTeam,
  useDeleteOrganization,
  useDeleteTeam,
  useOrganization,
  useOrganizationAudit,
  useOrganizationMembers,
  useOrganizationTeams,
  useRestoreOrganization,
  useUnarchiveTeam,
} from '@/hooks'

export function OrganizationSettingsPage(): ReactElement {
  const { id } = useParams({ from: '/_app/organizations/$id' })
  const org = useOrganization(id)
  const members = useOrganizationMembers(id)
  const teams = useOrganizationTeams(id)
  const audit = useOrganizationAudit(id)
  const delOrg = useDeleteOrganization(id)
  const restoreOrg = useRestoreOrganization(id)
  const archiveTeam = useArchiveTeam(id)
  const unarchiveTeam = useUnarchiveTeam(id)
  const deleteTeam = useDeleteTeam(id)
  const [confirmName, setConfirmName] = useState('')
  const [tab, setTab] = useState<string | null>('members')

  const canAdmin = org.data?.role === 'owner' || org.data?.role === 'admin'
  const pendingDelete = !!org.data?.deleted_at

  return (
    <Stack gap="xl">
      <Group gap="md">
        <ThemeIcon size={40} radius="md" variant="light" color="teal">
          <IconBuilding size={22} stroke={1.5} />
        </ThemeIcon>
        <div>
          <Title order={2}>{org.data?.name ?? 'Organization'}</Title>
          <Text size="sm" c="dimmed">
            Organization settings
          </Text>
        </div>
      </Group>

      {org.isError ? <QueryErrorAlert error={org.error} onRetry={() => void org.refetch()} /> : null}

      {pendingDelete ? (
        <Stack gap="sm">
          <Text>This organization is scheduled for deletion. Owners can restore it within 30 days.</Text>
          {org.data?.role === 'owner' ? (
            <Button
              loading={restoreOrg.isPending}
              onClick={() =>
                restoreOrg.mutate(undefined, {
                  onSuccess: () => notifications.show({ message: 'Organization restored' }),
                  onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                })
              }
            >
              Restore organization
            </Button>
          ) : null}
        </Stack>
      ) : null}

      {canAdmin && !pendingDelete ? (
        <Tabs value={tab} onChange={setTab}>
          <Tabs.List>
            <Tabs.Tab value="members">Members</Tabs.Tab>
            <Tabs.Tab value="teams">Teams</Tabs.Tab>
            <Tabs.Tab value="audit">Audit log</Tabs.Tab>
            <Tabs.Tab value="danger">Danger zone</Tabs.Tab>
          </Tabs.List>

          <Tabs.Panel value="members" pt="md">
            {members.isError ? (
              <QueryErrorAlert error={members.error} onRetry={() => void members.refetch()} />
            ) : (
              <Table striped highlightOnHover>
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>Email</Table.Th>
                    <Table.Th>Role</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {(members.data ?? []).map((m) => (
                    <Table.Tr key={m.user_id}>
                      <Table.Td>{m.display_name ? `${m.display_name} (${m.email})` : m.email}</Table.Td>
                      <Table.Td>{m.role}</Table.Td>
                    </Table.Tr>
                  ))}
                </Table.Tbody>
              </Table>
            )}
          </Tabs.Panel>

          <Tabs.Panel value="teams" pt="md">
            <Stack gap="sm">
              {(teams.data ?? []).map((t) => (
                <Group key={t.id} justify="space-between">
                  <Text>
                    {t.name}
                    {t.is_default ? ' (default)' : ''}
                    {t.archived_at ? ' — archived' : ''}
                  </Text>
                  <Group gap="xs">
                    {t.archived_at ? (
                      <Button
                        size="xs"
                        variant="light"
                        loading={unarchiveTeam.isPending}
                        onClick={() =>
                          unarchiveTeam.mutate(t.id, {
                            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                          })
                        }
                      >
                        Restore
                      </Button>
                    ) : (
                      <Button
                        size="xs"
                        variant="light"
                        loading={archiveTeam.isPending}
                        onClick={() =>
                          archiveTeam.mutate(t.id, {
                            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                          })
                        }
                      >
                        Archive
                      </Button>
                    )}
                    {!t.is_default ? (
                      <Button
                        size="xs"
                        color="red"
                        variant="light"
                        leftSection={<IconTrash size={14} />}
                        loading={deleteTeam.isPending}
                        onClick={() =>
                          deleteTeam.mutate(t.id, {
                            onSuccess: () => notifications.show({ message: 'Team deleted' }),
                            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                          })
                        }
                      >
                        Delete
                      </Button>
                    ) : null}
                  </Group>
                </Group>
              ))}
            </Stack>
          </Tabs.Panel>

          <Tabs.Panel value="audit" pt="md">
            {audit.isError ? (
              <QueryErrorAlert error={audit.error} onRetry={() => void audit.refetch()} />
            ) : (
              <Table striped highlightOnHover>
                <Table.Thead>
                  <Table.Tr>
                    <Table.Th>When</Table.Th>
                    <Table.Th>Action</Table.Th>
                    <Table.Th>Actor</Table.Th>
                  </Table.Tr>
                </Table.Thead>
                <Table.Tbody>
                  {(audit.data ?? []).map((e) => (
                    <Table.Tr key={e.id}>
                      <Table.Td>{new Date(e.created_at).toLocaleString()}</Table.Td>
                      <Table.Td>{e.action}</Table.Td>
                      <Table.Td>{e.actor_email ?? e.actor_user_id ?? '—'}</Table.Td>
                    </Table.Tr>
                  ))}
                </Table.Tbody>
              </Table>
            )}
          </Tabs.Panel>

          <Tabs.Panel value="danger" pt="md">
            {org.data?.role === 'owner' ? (
              <Stack gap="sm" maw={420}>
                <Text size="sm">Type the organization name to schedule deletion. You can restore within 30 days.</Text>
                <TextInput
                  label="Confirm name"
                  value={confirmName}
                  onChange={(e) => setConfirmName(e.currentTarget.value)}
                />
                <Button
                  color="red"
                  disabled={confirmName.trim() !== (org.data?.name ?? '')}
                  loading={delOrg.isPending}
                  onClick={() =>
                    delOrg.mutate(confirmName, {
                      onSuccess: () => notifications.show({ message: 'Organization scheduled for deletion' }),
                      onError: (e) => notifications.show({ color: 'red', message: String(e) }),
                    })
                  }
                >
                  Delete organization
                </Button>
              </Stack>
            ) : (
              <Text size="sm" c="dimmed">
                Only owners can delete the organization.
              </Text>
            )}
          </Tabs.Panel>
        </Tabs>
      ) : null}
    </Stack>
  )
}
