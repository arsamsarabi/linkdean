import type { ReactElement } from 'react'
import { Select } from '@mantine/core'
import type { CategoryVisibility } from '@/schemas'

const OPTIONS = [
  { value: 'private', label: 'Only me' },
  { value: 'followers', label: 'Followers' },
  { value: 'members', label: 'Members' },
]

type Props = {
  value: CategoryVisibility
  isVault: boolean
  onChange: (next: CategoryVisibility) => void
}

export function CategoryVisibilitySelect({ value, isVault, onChange }: Props): ReactElement {
  return (
    <Select
      label="Who can see this"
      description="Members means anyone signed in here. Use a Public link for people without an account."
      data={OPTIONS}
      value={value}
      allowDeselect={false}
      disabled={isVault}
      onChange={(next) => next && onChange(next as CategoryVisibility)}
    />
  )
}
