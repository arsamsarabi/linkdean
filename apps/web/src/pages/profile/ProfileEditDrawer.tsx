import type { ReactElement, ReactNode } from 'react'
import { Button, Drawer, Group, Select, Stack } from '@mantine/core'
import { useDiscardGuard } from '@/components/confirm-modal'
import { useDrawerClosesOnEscape, useDrawerSize } from '@/hooks/render'
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
  /** Unsaved edits in the drawer: closing asks before discarding them. */
  dirty: boolean
  children: ReactNode
}

export function ProfileEditDrawer({ opened, visibility, onVisibility, onClose, dirty, children }: Props): ReactElement {
  const closeOnEscape = useDrawerClosesOnEscape()
  const drawerSize = useDrawerSize('lg')
  const { requestClose, discardModal } = useDiscardGuard(dirty, onClose)
  return (
    <>
      <Drawer
        closeOnEscape={closeOnEscape}
        opened={opened}
        onClose={requestClose}
        position="right"
        size={drawerSize}
        title="Edit profile"
      >
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
            <Button variant="default" onClick={requestClose}>
              Cancel
            </Button>
          </Group>
        </Stack>
      </Drawer>
      {discardModal}
    </>
  )
}
