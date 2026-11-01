import { useState, type ReactElement } from 'react'
import { Button, Checkbox, Divider, Drawer, Group, Stack, Text, Textarea, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { ConfirmModal, QueryErrorAlert } from '@/components'
import { useExposedContent, useModerate, useModerationActions } from '@/hooks'
import {
  exposedGroups,
  MAX_BAN_REASON,
  moderateItems,
  moderationLabel,
  validBanReason,
  type ExposedGroup,
  type GroupAction,
} from '@/services'
import { SettingsSkeleton } from './SettingsSkeleton'

type Props = {
  user: { id: string; label: string }
  onClose: () => void
}

type Pending = { group: ExposedGroup; action: GroupAction }

/** What a banned user exposes to others, and the admin actions on it. */
export function ModerationDrawer({ user, onClose }: Props): ReactElement {
  const content = useExposedContent(user.id)
  const history = useModerationActions(user.id)
  const moderate = useModerate(user.id)
  const [note, setNote] = useState('')
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set())
  const [pending, setPending] = useState<Pending | null>(null)
  const groups = content.data ? exposedGroups(content.data.content, user.id) : []

  function toggle(id: string): void {
    setSelected((prev) => {
      const next = new Set(prev)
      if (!next.delete(id)) next.add(id)
      return next
    })
  }

  async function apply({ group, action }: Pending): Promise<void> {
    try {
      const { outcome } = await moderate.mutateAsync({
        reason: note.trim(),
        actions: moderateItems(group, action, selected),
      })
      const skipped = outcome.skipped > 0 ? `, ${outcome.skipped} already done` : ''
      notifications.show({ message: `${outcome.applied} changed${skipped}` })
      setSelected(new Set())
      setPending(null)
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  function start(group: ExposedGroup, action: GroupAction): void {
    if (action.confirm) setPending({ group, action })
    else void apply({ group, action })
  }

  return (
    <Drawer opened onClose={onClose} position="right" size="lg" title={`Exposed content: ${user.label}`}>
      <Stack gap="lg">
        <Textarea
          label="Note"
          description="Shown to the user and kept in the history."
          required
          rows={2}
          maxLength={MAX_BAN_REASON}
          value={note}
          onChange={(e) => setNote(e.currentTarget.value)}
        />
        {content.isError ? <QueryErrorAlert error={content.error} onRetry={() => void content.refetch()} /> : null}
        {content.isLoading ? <SettingsSkeleton /> : null}
        {content.data && groups.length === 0 ? (
          <Text size="sm" c="dimmed">
            Nothing is exposed to other people.
          </Text>
        ) : null}
        {groups.map((group) => {
          const count = group.rows.filter((r) => selected.has(r.id)).length
          return (
            <Stack key={group.key} gap="xs">
              <Title order={5}>
                {group.title} ({group.rows.length})
              </Title>
              {group.rows.map((row) => (
                <Checkbox
                  key={row.id}
                  label={row.label}
                  description={row.detail}
                  checked={selected.has(row.id)}
                  onChange={() => toggle(row.id)}
                  styles={{ label: { overflowWrap: 'anywhere' }, description: { overflowWrap: 'anywhere' } }}
                />
              ))}
              <Group gap="xs">
                {group.actions.map((action) => (
                  <Button
                    key={action.label}
                    size="xs"
                    variant="default"
                    disabled={count === 0 || !validBanReason(note)}
                    loading={moderate.isPending}
                    onClick={() => start(group, action)}
                  >
                    {action.label}
                  </Button>
                ))}
              </Group>
              <Divider />
            </Stack>
          )
        })}
        <Stack gap="xs">
          <Title order={5}>History</Title>
          {(history.data?.actions ?? []).map((a) => (
            <div key={a.id}>
              <Text size="sm" style={{ overflowWrap: 'anywhere' }}>
                {moderationLabel(a.action)}
                {a.subject_title ? `: ${a.subject_title}` : ''}
              </Text>
              <Text size="xs" c="dimmed">
                {new Date(a.created_at).toLocaleString()} · {a.admin_email ?? 'Former admin'}
                {a.reason ? ` · ${a.reason}` : ''}
              </Text>
            </div>
          ))}
        </Stack>
      </Stack>
      <ConfirmModal
        opened={!!pending}
        onClose={() => setPending(null)}
        onConfirm={() => pending && void apply(pending)}
        title={pending?.action.label ?? ''}
        body={pending?.action.confirm ?? ''}
        confirmLabel={pending?.action.label}
        loading={moderate.isPending}
      />
    </Drawer>
  )
}
