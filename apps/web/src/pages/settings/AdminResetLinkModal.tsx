import type { ReactElement } from 'react'
import { ActionIcon, Alert, Button, CopyButton, Group, Modal, Stack, Text, TextInput, Tooltip } from '@mantine/core'
import { IconCheck, IconCopy, IconKey } from '@tabler/icons-react'
import { useBootstrap, useCreateResetLink } from '@/hooks'

type Props = {
  user: { id: string; label: string }
  onClose: () => void
}

/** Creates a 24-hour reset link for an account, only when the admin asks for it. */
export function AdminResetLinkModal({ user, onClose }: Props): ReactElement {
  const create = useCreateResetLink()
  const mailEnabled = useBootstrap().data?.mail_enabled ?? false
  const link = create.data

  return (
    <Modal opened onClose={onClose} title={`Reset link for ${user.label}`}>
      <Stack>
        {link ? (
          <>
            <TextInput
              label="Reset link"
              readOnly
              value={link.url}
              onFocus={(e) => e.currentTarget.select()}
              rightSection={
                <CopyButton value={link.url}>
                  {({ copied, copy }) => (
                    <Tooltip label={copied ? 'Copied' : 'Copy'} withArrow>
                      <ActionIcon
                        variant="subtle"
                        color={copied ? 'teal' : 'gray'}
                        onClick={copy}
                        aria-label="Copy reset link"
                      >
                        {copied ? <IconCheck size={16} /> : <IconCopy size={16} />}
                      </ActionIcon>
                    </Tooltip>
                  )}
                </CopyButton>
              }
            />
            <Text size="sm">
              Send it privately. Anyone with this link can set the password. It works once, for {link.expires_in_hours}{' '}
              hours. Creating a new link cancels the previous one.
            </Text>
            {mailEnabled ? (
              <Text size="sm" c="dimmed">
                We also emailed them that a link was created.
              </Text>
            ) : null}
            <Group justify="flex-end">
              <Button onClick={onClose}>Done</Button>
            </Group>
          </>
        ) : (
          <>
            <Text size="sm">
              Create a link that lets {user.label} set a new password. It works once, for 24 hours, and cancels any
              earlier link.
            </Text>
            {create.isError ? (
              <Alert color="red" variant="light">
                {create.error.message}
              </Alert>
            ) : null}
            <Group justify="flex-end">
              <Button variant="default" onClick={onClose}>
                Cancel
              </Button>
              <Button
                leftSection={<IconKey size={16} stroke={1.5} />}
                loading={create.isPending}
                onClick={() => create.mutate(user.id)}
              >
                Create link
              </Button>
            </Group>
          </>
        )}
      </Stack>
    </Modal>
  )
}
