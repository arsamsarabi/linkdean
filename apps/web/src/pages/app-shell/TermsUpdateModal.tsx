import { type ReactElement } from 'react'
import { Alert, Anchor, Button, Group, Modal, Stack, Text } from '@mantine/core'
import { IconFileText } from '@tabler/icons-react'
import { useAcceptTerms } from '@/hooks'
import { TERMS_VERSION } from '@/lib'

type Props = {
  /** The account accepted an older version of the terms, or none. */
  outdated: boolean
  onSignOut: () => void
}

/** Asks for the current Terms of Service after they change. It cannot be dismissed: accept or sign out. */
export function TermsUpdateModal({ outdated, onSignOut }: Props): ReactElement | null {
  const accept = useAcceptTerms()
  if (!outdated) return null

  return (
    <Modal
      opened
      onClose={() => {}}
      withCloseButton={false}
      closeOnClickOutside={false}
      closeOnEscape={false}
      title={
        <Group gap="xs">
          <IconFileText size={18} stroke={1.5} />
          <Text fw={600}>Our terms changed</Text>
        </Group>
      }
    >
      <Stack>
        <Text size="sm">
          We updated the Terms of Service (version {TERMS_VERSION}). Read them, then accept to keep using Linkdean.
        </Text>
        <Group gap="md">
          <Anchor href="/terms" target="_blank" rel="noopener" size="sm">
            Terms of Service
          </Anchor>
          <Anchor href="/privacy" target="_blank" rel="noopener" size="sm">
            Privacy Policy
          </Anchor>
          <Anchor href="/acceptable-use" target="_blank" rel="noopener" size="sm">
            Acceptable Use Policy
          </Anchor>
        </Group>
        <Text size="xs" c="dimmed">
          If you do not accept, sign out. You can still sign in later to export your data or delete your account.
        </Text>
        {accept.isError ? (
          <Alert color="red" variant="light">
            {accept.error.message}
          </Alert>
        ) : null}
        <Group justify="flex-end">
          <Button variant="default" onClick={onSignOut}>
            Sign out
          </Button>
          <Button loading={accept.isPending} onClick={() => accept.mutate(TERMS_VERSION)} data-autofocus>
            Accept and continue
          </Button>
        </Group>
      </Stack>
    </Modal>
  )
}
