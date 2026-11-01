import type { ReactElement, ReactNode } from 'react'
import { Button, Drawer, Group, Select, Stack } from '@mantine/core'
import type { ProfileVisibility } from '@/schemas'

const OPTIONS = [
  { value: 'hidden', label: 'Hidden' },
  { value: 'followers', label: 'Followers' },
  { value: 'members', label: 'Members' },
]

type Props = {
  opened: boolean
  visibility: ProfileVisibility
  onVisibility: (next: ProfileVisibility) => void
  onClose: () => void
  children: ReactNode
}

export function ProfileEditDrawer({ opened, visibility, onVisibility, onClose, children }: Props): ReactElement {
  return (
    <Drawer opened={opened} onClose={onClose} position="right" size="lg" title="Edit profile">
      <Stack gap="xl">
        <Select
          label="Who can see my profile"
          description="Members means anyone signed in here. Use a Public link for people without an account."
          data={OPTIONS}
          value={visibility}
          allowDeselect={false}
          onChange={(next) => next && onVisibility(next as ProfileVisibility)}
        />
        {children}
        <Group justify="flex-end">
          <Button variant="default" onClick={onClose}>
            Cancel
          </Button>
        </Group>
      </Stack>
    </Drawer>
  )
}
