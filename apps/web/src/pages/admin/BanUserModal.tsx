import { useState, type ReactElement } from 'react'
import { Button, Group, Modal, Stack, Textarea } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useBanUser } from '@/hooks'
import { banDurationBody, emptyBanDuration, MAX_BAN_REASON, validBanReason } from '@/services'
import { BanDurationFields } from './BanDurationFields'

type Props = {
  user: { id: string; label: string }
  onClose: () => void
}

export function BanUserModal({ user, onClose }: Props): ReactElement {
  const ban = useBanUser()
  const [reason, setReason] = useState('')
  const [duration, setDuration] = useState(emptyBanDuration)
  const body = banDurationBody(duration)

  async function submit(): Promise<void> {
    if (!body) return
    const { days, ends_at } = body
    try {
      await ban.mutateAsync({ id: user.id, reason: reason.trim(), days, ends_at })
      notifications.show({ message: `${user.label} banned` })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Modal opened onClose={onClose} title={`Ban ${user.label}`}>
      <Stack>
        <Textarea
          label="Reason"
          description="Shown to the user."
          required
          rows={3}
          maxLength={MAX_BAN_REASON}
          value={reason}
          onChange={(e) => setReason(e.currentTarget.value)}
        />
        <BanDurationFields value={duration} onChange={setDuration} />
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
          <Button
            color="red"
            disabled={!validBanReason(reason) || !body}
            loading={ban.isPending}
            onClick={() => void submit()}
          >
            Ban
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
