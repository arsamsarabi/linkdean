import { useState, type ReactElement } from 'react'
import { Badge, Button, Divider, Group, Loader, Paper, Stack, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { QueryErrorAlert, REPORT_REASONS } from '@/components'
import { useAbuseReports, usePlatformSettings, useResolveAbuseReport, useUpdatePlatformSettings } from '@/hooks'
import type { AbuseReport } from '@/schemas'
import { ResolveReportModal } from './ResolveReportModal'

const TARGET_LABELS: Record<AbuseReport['target_type'], string> = {
  public_list: 'Public list',
  profile: 'Profile',
  comment: 'Comment',
}

function reasonLabel(reason: string): string {
  return REPORT_REASONS.find((r) => r.value === reason)?.label ?? reason
}

/** Where people can reach the operator about abuse. Shown in the report form. */
function AbuseContactField(): ReactElement | null {
  const platform = usePlatformSettings(true)
  const save = useUpdatePlatformSettings()
  const saved = platform.data?.settings.abuse_contact_email ?? ''
  const [draft, setDraft] = useState<string | null>(null)
  if (!platform.data) return null
  const value = draft ?? saved
  const changed = value.trim() !== saved

  return (
    <Group align="flex-end" gap="sm" wrap="nowrap">
      <TextInput
        flex={1}
        type="email"
        label="Abuse contact email"
        description="Shown in the report form so people can write to you. Leave blank to hide it."
        value={value}
        onChange={(e) => setDraft(e.currentTarget.value)}
      />
      <Button
        loading={save.isPending}
        disabled={!changed}
        onClick={() =>
          save.mutate(
            { abuse_contact_email: value.trim() || null },
            {
              onSuccess: () => {
                setDraft(null)
                notifications.show({ message: 'Abuse contact saved' })
              },
              onError: (err) => notifications.show({ color: 'red', message: String(err) }),
            }
          )
        }
      >
        Save
      </Button>
    </Group>
  )
}

/** The open abuse reports, with a contact setting for the operator. */
export function AdminReportsPanel(): ReactElement {
  const reports = useAbuseReports('open')
  const resolve = useResolveAbuseReport()
  const [asking, setAsking] = useState<{ report: AbuseReport; action: 'warn' | 'ban' } | null>(null)
  const rows = reports.data?.reports ?? []

  function act(report: AbuseReport, action: 'dismiss' | 'hide'): void {
    resolve.mutate(
      { id: report.id, action },
      {
        onSuccess: () => notifications.show({ message: action === 'hide' ? 'Content hidden' : 'Report dismissed' }),
        onError: (err) => notifications.show({ color: 'red', message: String(err) }),
      }
    )
  }

  return (
    <Paper p="xl" withBorder>
      <Stack gap="md">
        <div>
          <Text fw={500}>Abuse reports</Text>
          <Text size="sm" c="dimmed">
            Signed-in users report public lists, profiles and comments. Hide takes the content down; Warn and Ban act on
            its owner.
          </Text>
        </div>
        <AbuseContactField />
        {reports.isError ? (
          <QueryErrorAlert
            error={reports.error}
            onRetry={() => void reports.refetch()}
            title="Reports failed to load"
          />
        ) : null}
        {reports.isLoading ? <Loader size="sm" /> : null}
        {reports.data && rows.length === 0 ? (
          <Text size="sm" c="dimmed">
            No open reports.
          </Text>
        ) : null}
        {rows.map((report, i) => (
          <Stack key={report.id} gap="xs">
            <Group gap="xs" wrap="wrap">
              <Badge variant="light">{TARGET_LABELS[report.target_type]}</Badge>
              <Badge variant="outline" color="gray">
                {reasonLabel(report.reason)}
              </Badge>
              <Text size="xs" c="dimmed">
                {new Date(report.created_at).toLocaleString()}
              </Text>
            </Group>
            <Text size="sm" fw={500} style={{ overflowWrap: 'anywhere' }}>
              {report.target_label || report.target_id}
            </Text>
            {report.details ? (
              <Text size="sm" style={{ whiteSpace: 'pre-wrap' }}>
                {report.details}
              </Text>
            ) : null}
            <Text size="xs" c="dimmed">
              Reported by {report.reporter_email ?? 'a deleted account'}
            </Text>
            <Group gap="xs" wrap="wrap">
              <Button size="xs" variant="default" disabled={resolve.isPending} onClick={() => act(report, 'dismiss')}>
                Dismiss
              </Button>
              <Button size="xs" variant="default" disabled={resolve.isPending} onClick={() => act(report, 'hide')}>
                Hide content
              </Button>
              <Button
                size="xs"
                variant="default"
                disabled={!report.target_user_id}
                onClick={() => setAsking({ report, action: 'warn' })}
              >
                Warn
              </Button>
              <Button
                size="xs"
                color="red"
                variant="light"
                disabled={!report.target_user_id}
                onClick={() => setAsking({ report, action: 'ban' })}
              >
                Ban
              </Button>
            </Group>
            {i < rows.length - 1 ? <Divider mt="xs" /> : null}
          </Stack>
        ))}
      </Stack>
      {asking ? (
        <ResolveReportModal report={asking.report} action={asking.action} onClose={() => setAsking(null)} />
      ) : null}
    </Paper>
  )
}
