import { useEffect, useState, type ReactElement } from 'react'
import { Button, Stack, Text, TextInput, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { useAcceptOrganizationInviteLink } from '@/hooks'

export function OrganizationJoinPage(): ReactElement {
  const search = useSearch({ from: '/_app/org-join' })
  const navigate = useNavigate()
  const accept = useAcceptOrganizationInviteLink()
  const [code, setCode] = useState(search.code ?? '')

  useEffect(() => {
    if (search.code) setCode(search.code)
  }, [search.code])

  return (
    <Stack gap="md" maw={480}>
      <Title order={2}>Join an organization</Title>
      <Text size="sm" c="dimmed">
        Paste an invite link code. Your email must be verified.
      </Text>
      <TextInput
        label="Invite code"
        value={code}
        onChange={(e) => setCode(e.currentTarget.value)}
        placeholder="ldoi_…"
      />
      <Button
        loading={accept.isPending}
        disabled={!code.trim()}
        onClick={() =>
          accept.mutate(code.trim(), {
            onSuccess: (org) => {
              notifications.show({ message: `Joined ${org.name}` })
              void navigate({ to: '/organizations/$id', params: { id: org.id } })
            },
            onError: (e) => notifications.show({ color: 'red', message: String(e) }),
          })
        }
      >
        Accept invite
      </Button>
    </Stack>
  )
}
