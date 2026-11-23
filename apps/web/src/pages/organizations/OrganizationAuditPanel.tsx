import { useMemo, useState, type ReactElement } from 'react'
import { Button, Group, Select, Stack, Table, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { QueryErrorAlert } from '@/components'
import {
  downloadOrganizationAuditCsv,
  useOrganizationAudit,
  useOrganizationMembers,
  type OrganizationAuditFilters,
} from '@/hooks'

function dayStartIso(dateYmd: string): string | undefined {
  const t = dateYmd.trim()
  if (!t) return undefined
  return `${t}T00:00:00.000Z`
}

function dayEndIso(dateYmd: string): string | undefined {
  const t = dateYmd.trim()
  if (!t) return undefined
  return `${t}T23:59:59.999Z`
}

export function OrganizationAuditPanel({ organizationId }: { organizationId: string }): ReactElement {
  const members = useOrganizationMembers(organizationId)
  const [actorUserId, setActorUserId] = useState<string | null>(null)
  const [action, setAction] = useState('')
  const [fromDay, setFromDay] = useState('')
  const [toDay, setToDay] = useState('')
  const [exporting, setExporting] = useState(false)

  const filters: OrganizationAuditFilters = useMemo(
    () => ({
      actorUserId: actorUserId ?? undefined,
      action: action.trim() || undefined,
      from: dayStartIso(fromDay),
      to: dayEndIso(toDay),
    }),
    [actorUserId, action, fromDay, toDay]
  )

  const audit = useOrganizationAudit(organizationId, filters)

  const actorOptions = useMemo(
    () =>
      (members.data ?? []).map((m) => ({
        value: m.user_id,
        label: m.display_name ? `${m.display_name} (${m.email})` : m.email,
      })),
    [members.data]
  )

  async function onExport(): Promise<void> {
    setExporting(true)
    try {
      await downloadOrganizationAuditCsv(organizationId, filters)
      notifications.show({ message: 'Audit CSV downloaded' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setExporting(false)
    }
  }

  return (
    <Stack gap="md">
      <Text size="sm" c="dimmed">
        Membership, teams, domains, SSO/SCIM and content moves into or out of the organization. Append-only; kept for
        one year.
      </Text>
      <Group align="flex-end" grow preventGrowOverflow wrap="wrap">
        <Select
          label="Person"
          placeholder="Anyone"
          clearable
          searchable
          data={actorOptions}
          value={actorUserId}
          onChange={setActorUserId}
          style={{ minWidth: 200 }}
        />
        <TextInput
          label="Action"
          placeholder="Exact action key"
          value={action}
          onChange={(e) => setAction(e.currentTarget.value)}
          style={{ minWidth: 160 }}
        />
        <TextInput
          label="From"
          type="date"
          value={fromDay}
          onChange={(e) => setFromDay(e.currentTarget.value)}
          style={{ minWidth: 140 }}
        />
        <TextInput
          label="To"
          type="date"
          value={toDay}
          onChange={(e) => setToDay(e.currentTarget.value)}
          style={{ minWidth: 140 }}
        />
        <Button variant="light" loading={exporting} onClick={() => void onExport()}>
          Download CSV
        </Button>
      </Group>
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
      {audit.isSuccess && (audit.data?.length ?? 0) === 0 ? (
        <Text size="sm" c="dimmed">
          No events match these filters.
        </Text>
      ) : null}
    </Stack>
  )
}
