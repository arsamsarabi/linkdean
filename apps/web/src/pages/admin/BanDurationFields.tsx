import type { ReactElement } from 'react'
import { Group, NumberInput, Radio, Stack, TextInput } from '@mantine/core'
import { MAX_BAN_DAYS, type BanDurationForm } from '@/services'

type Props = {
  value: BanDurationForm
  onChange: (next: BanDurationForm) => void
}

export function BanDurationFields({ value, onChange }: Props): ReactElement {
  return (
    <Stack gap="sm">
      <Radio.Group
        label="Duration"
        value={value.mode}
        onChange={(mode) => onChange({ ...value, mode: mode as BanDurationForm['mode'] })}
      >
        <Group mt="xs">
          <Radio value="permanent" label="Permanent" />
          <Radio value="days" label="Number of days" />
          <Radio value="date" label="Until a date" />
        </Group>
      </Radio.Group>
      {value.mode === 'days' ? (
        <NumberInput
          label="Days"
          min={1}
          max={MAX_BAN_DAYS}
          allowDecimal={false}
          value={value.days}
          onChange={(days) => onChange({ ...value, days })}
        />
      ) : null}
      {value.mode === 'date' ? (
        <TextInput
          label="Last day of the ban"
          type="date"
          value={value.date}
          onChange={(e) => onChange({ ...value, date: e.currentTarget.value })}
        />
      ) : null}
    </Stack>
  )
}
