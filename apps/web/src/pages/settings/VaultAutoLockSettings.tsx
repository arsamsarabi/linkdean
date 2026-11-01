import { useState, type ReactElement } from 'react'
import { Checkbox, NumberInput, Paper, Stack } from '@mantine/core'

const DEFAULT_AUTO_LOCK_MINUTES = 15

type Props = {
  minutes: number
  saving: boolean
  onChange: (minutes: number) => void
}

export function VaultAutoLockSettings({ minutes, saving, onChange }: Props): ReactElement {
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
