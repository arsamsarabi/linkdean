import { useState, type ReactElement } from 'react'
import { Alert, Anchor, Button, Drawer, Radio, Stack, Text, Textarea } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useCreateReport, useReportConfig } from '@/hooks'
import { useDrawerClosesOnEscape } from '@/hooks/render'
import type { AbuseReason, AbuseTargetType } from '@/schemas'
import { REPORT_DETAILS_MAX, REPORT_REASONS } from './reasons'

type Props = {
  opened: boolean
  onClose: () => void
  targetType: AbuseTargetType
  /** A public list's slug, a user id or a comment id. */
  targetId: string
  /** What is being reported, for the heading. */
  subject: string
}

/** Reason and optional details for an abuse report. Signed-in users only. */
export function ReportDrawer({ opened, onClose, targetType, targetId, subject }: Props): ReactElement {
  const closeOnEscape = useDrawerClosesOnEscape()
  const create = useCreateReport()
  const config = useReportConfig(opened)
  const [reason, setReason] = useState<AbuseReason | null>(null)
  const [details, setDetails] = useState('')
  const [error, setError] = useState<string | null>(null)
  const contact = config.data?.abuse_contact_email

  async function submit(): Promise<void> {
    if (!reason) return
    setError(null)
    try {
      await create.mutateAsync({
        target_type: targetType,
        target_id: targetId,
        reason,
        details: details.trim() || undefined,
      })
      notifications.show({ message: 'Report sent. Thank you.' })
      setReason(null)
      setDetails('')
      onClose()
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e))
    }
  }

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      closeOnEscape={closeOnEscape}
      position="right"
      size="md"
      padding="lg"
      title={`Report ${subject}`}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault()
          void submit()
        }}
      >
        <Stack gap="md">
          <Text size="sm" c="dimmed">
            An admin of this instance reviews every report. The person you report is not told who reported them.
          </Text>
          <Radio.Group
            label="Why are you reporting this?"
            required
            value={reason}
            onChange={(v) => setReason(v as AbuseReason)}
          >
            <Stack gap="xs" mt="xs">
              {REPORT_REASONS.map((r) => (
                <Radio key={r.value} value={r.value} label={r.label} />
              ))}
            </Stack>
          </Radio.Group>
          <Textarea
            label="Details (optional)"
            maxLength={REPORT_DETAILS_MAX}
            autosize
            minRows={3}
            maxRows={8}
            value={details}
            onChange={(e) => setDetails(e.currentTarget.value)}
          />
          {error ? (
            <Alert color="red" role="alert">
              {error}
            </Alert>
          ) : null}
          <Button type="submit" disabled={!reason} loading={create.isPending}>
            Send report
          </Button>
          {contact ? (
            <Text size="sm" c="dimmed">
              To reach the operator directly, email <Anchor href={`mailto:${contact}`}>{contact}</Anchor>.
            </Text>
          ) : null}
        </Stack>
      </form>
    </Drawer>
  )
}
