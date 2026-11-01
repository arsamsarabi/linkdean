import { useState, type ReactElement } from 'react'
import { Button, Group, Modal, Stack, Textarea } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useChangeBans } from '@/hooks'
import { banDurationBody, emptyBanDuration, MAX_BAN_REASON, validBanReason } from '@/services'
import { BanDurationFields } from './BanDurationFields'

type Props = {
  mode: 'reason' | 'duration'
  userIds: string[]
  onDone: () => void
  onClose: () => void
}

/** Bulk edit of one field across the selected bans. */
export function BanEditModal({ mode, userIds, onDone, onClose }: Props): ReactElement {
  const change = useChangeBans()
  const [reason, setReason] = useState('')
  const [duration, setDuration] = useState(emptyBanDuration)
  const body = banDurationBody(duration)
  const valid = mode === 'reason' ? validBanReason(reason) : !!body
  const count = userIds.length

  async function submit(): Promise<void> {
    const patch = mode === 'reason' ? { reason: reason.trim() } : body
    try {
      await change.mutateAsync({ user_ids: userIds, ...patch })
      notifications.show({ message: `${count} ban${count === 1 ? '' : 's'} changed` })
      onDone()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title={mode === 'reason' ? 'Change reason' : 'Change duration'}>
      <Stack>
        {mode === 'reason' ? (
          <Textarea
            label="Reason"
            description="Shown to each selected user."
            required
            rows={3}
            maxLength={MAX_BAN_REASON}
            value={reason}
            onChange={(e) => setReason(e.currentTarget.value)}
          />
        ) : (
          <BanDurationFields value={duration} onChange={setDuration} />
        )}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          <Button disabled={!valid} loading={change.isPending} onClick={() => void submit()}>
            Apply to {count}
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
