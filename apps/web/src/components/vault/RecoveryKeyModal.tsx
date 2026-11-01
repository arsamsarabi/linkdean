import { useState, type ReactElement } from 'react'
import { Alert, Button, Checkbox, Code, CopyButton, Group, Modal, Stack, Text } from '@mantine/core'

type Props = {
  recoveryKey: string
  onDone: () => void
}

/** Shows a recovery key once. The server keeps only a wrapped copy of the vault key. */
export function RecoveryKeyModal({ recoveryKey, onDone }: Props): ReactElement {
  const [saved, setSaved] = useState(false)
  return (
    <Modal
      opened
      onClose={() => {
        if (saved) onDone()
      }}
      title="Save your recovery key"
      withCloseButton={false}
      closeOnClickOutside={false}
      closeOnEscape={false}
    >
      <Stack>
        <Alert color="yellow" variant="light">
          This is shown once. Without this key or your passphrase, nobody can open this vault, including us.
        </Alert>
        <Code block>{recoveryKey}</Code>
        <Group justify="space-between">
          <CopyButton value={recoveryKey}>
            {({ copied, copy }) => (
              <Button variant="default" onClick={copy}>
                {copied ? 'Copied' : 'Copy'}
              </Button>
            )}
          </CopyButton>
          <Text size="xs" c="dimmed">
            Keep it somewhere safe, away from this device.
          </Text>
        </Group>
        <Checkbox
          label="I saved this recovery key"
          checked={saved}
          onChange={(e) => setSaved(e.currentTarget.checked)}
        />
        <Button disabled={!saved} onClick={onDone}>
          Done
        </Button>
      </Stack>
    </Modal>
  )
}
