import { useState, type ReactElement } from 'react'
import { Button, Group, Modal, Stack, Text, Textarea } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useResolveAbuseReport } from '@/hooks'
import type { AbuseReport } from '@/schemas'

type Props = {
  report: AbuseReport
  action: 'warn' | 'ban'
  onClose: () => void
}

const COPY = {
  warn: {
    title: 'Warn the user',
    hint: 'The user gets this message by email when outbound email is on.',
    label: 'Warning',
    submit: 'Send warning',
    needsNote: true,
  },
  ban: {
    title: 'Ban the user',
    hint: 'The ban is permanent until you lift it in Banned users. Leave the reason blank to use the report reason.',
    label: 'Ban reason',
    submit: 'Ban',
    needsNote: false,
  },
} as const

/** Asks for the note a warning or a ban carries, then resolves the report. */
export function ResolveReportModal({ report, action, onClose }: Props): ReactElement {
  const resolve = useResolveAbuseReport()
  const [note, setNote] = useState('')
  const copy = COPY[action]
  const trimmed = note.trim()

  async function submit(): Promise<void> {
    try {
      await resolve.mutateAsync({ id: report.id, action, note: trimmed || undefined })
      notifications.show({ message: action === 'warn' ? 'Warning recorded' : 'User banned' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title={copy.title} centered>
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          {copy.hint}
        </Text>
        <Textarea
          label={copy.label}
          required={copy.needsNote}
          maxLength={500}
          autosize
          minRows={3}
          value={note}
          onChange={(e) => setNote(e.currentTarget.value)}
          data-autofocus
        />
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          <Button
            color={action === 'ban' ? 'red' : undefined}
            loading={resolve.isPending}
            disabled={copy.needsNote && !trimmed}
            onClick={() => void submit()}
          >
            {copy.submit}
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
