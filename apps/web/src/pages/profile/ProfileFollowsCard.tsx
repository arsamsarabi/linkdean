import { type ReactElement } from 'react'
import { Card, Group, Stack, Text, Title } from '@mantine/core'
import { QueryErrorAlert } from '@/components'
import { useMyFollowers, useMyFollowing } from '@/hooks'

export function ProfileFollowsCard(): ReactElement {
  const followers = useMyFollowers()
  const following = useMyFollowing()
  const followerUsers = followers.data?.users ?? []
  const followingUsers = following.data?.users ?? []

  return (
    <Card withBorder padding="lg" radius="md">
      <Stack gap="md">
        <Title order={4}>Following</Title>
        <Group gap="xl">
          <Text size="sm">
            <Text span fw={600}>
              {followerUsers.length}
            </Text>{' '}
            followers
          </Text>
          <Text size="sm">
            <Text span fw={600}>
              {followingUsers.length}
            </Text>{' '}
            following
          </Text>
        </Group>

        {followers.isError ? (
          <QueryErrorAlert error={followers.error} onRetry={() => void followers.refetch()} />
        ) : null}
        {following.isError ? (
          <QueryErrorAlert error={following.error} onRetry={() => void following.refetch()} />
        ) : null}

        <Stack gap={4}>
          <Text size="sm" fw={500}>
            People you follow
          </Text>
          {following.isLoading ? (
            <Text size="sm" c="dimmed">
              Loading…
            </Text>
          ) : followingUsers.length === 0 ? (
            <Text size="sm" c="dimmed">
              Nobody yet — use ⋮ on a shared bookmark to follow its owner.
            </Text>
          ) : (
            followingUsers.map((u) => (
              <Text key={u.id} size="sm">
                {u.display_name}
              </Text>
            ))
          )}
        </Stack>

        <Stack gap={4}>
          <Text size="sm" fw={500}>
            Followers
          </Text>
          {followers.isLoading ? (
            <Text size="sm" c="dimmed">
              Loading…
            </Text>
          ) : followerUsers.length === 0 ? (
            <Text size="sm" c="dimmed">
              No followers yet.
            </Text>
          ) : (
            followerUsers.map((u) => (
              <Text key={u.id} size="sm">
                {u.display_name}
              </Text>
            ))
          )}
        </Stack>
      </Stack>
    </Card>
  )
}
