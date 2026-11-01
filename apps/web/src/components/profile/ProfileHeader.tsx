import type { ReactElement } from 'react'
import { Avatar, Button, Group, Stack, Text, Title, UnstyledButton } from '@mantine/core'
import { useAvatarImage } from '@/hooks'
import type { Profile } from '@/schemas'

export type FollowListKind = 'followers' | 'following'

type Props = {
  profile: Profile
  followPending?: boolean
  avatarBust?: number
  onToggleFollow: () => void
  onEdit: () => void
  onOpenList: (kind: FollowListKind) => void
}

export function ProfileHeader({
  profile,
  followPending,
  avatarBust,
  onToggleFollow,
  onEdit,
  onOpenList,
}: Props): ReactElement {
  const src = useAvatarImage(profile.has_avatar, avatarBust, profile.is_self ? 'me' : profile.id)
  const { followers, following_count: followingCount, following } = profile.follow

  return (
    <Group align="flex-start" justify="space-between" wrap="nowrap" gap="lg">
      <Group align="flex-start" gap="lg" wrap="nowrap">
        <Avatar src={src} size={80} radius="xl" alt="" name={profile.display_name} />
        <Stack gap={4}>
          <Title order={2}>{profile.display_name}</Title>
          {profile.email ? (
            <Text size="sm" c="dimmed">
              {profile.email}
            </Text>
          ) : null}
          {profile.bio ? <Text size="sm">{profile.bio}</Text> : null}
          <Group gap="lg">
            <UnstyledButton onClick={() => onOpenList('followers')}>
              <Text size="sm">{`${followers} follower${followers === 1 ? '' : 's'}`}</Text>
            </UnstyledButton>
            <UnstyledButton onClick={() => onOpenList('following')}>
              <Text size="sm">{`${followingCount} following`}</Text>
            </UnstyledButton>
          </Group>
        </Stack>
      </Group>
      {profile.is_self ? (
        <Button variant="default" onClick={onEdit}>
          Edit profile
        </Button>
      ) : (
        <Button variant={following ? 'default' : 'filled'} loading={followPending} onClick={onToggleFollow}>
          {following ? 'Unfollow' : 'Follow'}
        </Button>
      )}
    </Group>
  )
}
