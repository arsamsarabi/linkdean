import type { ReactElement } from 'react'
import { Anchor, Loader, Modal, Stack, Text } from '@mantine/core'
import { Link } from '@tanstack/react-router'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { useProfileFollows } from '@/hooks'
import type { FollowListKind } from './ProfileHeader'

type Props = {
  userId: string
  kind: FollowListKind | null
  onClose: () => void
}

export function FollowListModal({ userId, kind, onClose }: Props): ReactElement {
  const list = useProfileFollows(userId, kind ?? 'followers', kind !== null)
  const users = list.data?.users ?? []

  return (
    <Modal opened={kind !== null} onClose={onClose} title={kind === 'following' ? 'Following' : 'Followers'}>
      {list.isLoading ? <Loader size="sm" /> : null}
      {list.isError ? <QueryErrorAlert error={list.error} onRetry={() => void list.refetch()} /> : null}
      {list.isSuccess && users.length === 0 ? <Text c="dimmed">Nobody to show.</Text> : null}
      <Stack gap="xs">
        {users.map((u) => (
          <Anchor key={u.id} component={Link} to={`/u/${u.id}`} onClick={onClose}>
            {u.display_name}
          </Anchor>
        ))}
      </Stack>
    </Modal>
  )
}
