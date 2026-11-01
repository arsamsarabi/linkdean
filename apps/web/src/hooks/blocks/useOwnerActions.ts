import { notifications } from '@mantine/notifications'
import { useFollowStatus, useFollowUser, useUnfollowUser } from '@/hooks/follows'
import { useBlockUser, useMuteUser, useMyMuted, useUnmuteUser } from './useBlocks'

type OwnerActions = {
  following: boolean
  muted: boolean
  blocking: boolean
  toggleFollow: () => Promise<void>
  toggleMute: () => Promise<void>
  block: () => Promise<boolean>
}

/**
 * Follow / mute / block actions for another user's item. Muted state comes from one
 * shared list, so a page of cards costs a single request for it.
 *
 * @example
 * const owner = useOwnerActions(b.owner_user_id, b.owner_display_name, isSomeoneElses)
 */
export function useOwnerActions(
  ownerId: string | null | undefined,
  ownerLabel: string | null | undefined,
  enabled: boolean
): OwnerActions {
  const followStatus = useFollowStatus(enabled ? ownerId : null, enabled)
  const follow = useFollowUser()
  const unfollow = useUnfollowUser()
  const mutedList = useMyMuted(enabled)
  const mute = useMuteUser()
  const unmute = useUnmuteUser()
  const blockUser = useBlockUser()
  const following = !!followStatus.data?.follow.following
  const muted = !!mutedList.data?.users.some((u) => u.id === ownerId)
  const label = ownerLabel || 'user'

  async function toggleFollow(): Promise<void> {
    if (!ownerId) return
    try {
      if (following) {
        await unfollow.mutateAsync(ownerId)
        notifications.show({ message: 'Unfollowed' })
      } else {
        await follow.mutateAsync(ownerId)
        notifications.show({ message: `Following ${label}` })
      }
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function toggleMute(): Promise<void> {
    if (!ownerId) return
    try {
      if (muted) {
        await unmute.mutateAsync(ownerId)
        notifications.show({ message: `Unmuted ${label}` })
      } else {
        await mute.mutateAsync(ownerId)
        notifications.show({ message: `Muted ${label}` })
      }
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function block(): Promise<boolean> {
    if (!ownerId) return false
    try {
      await blockUser.mutateAsync(ownerId)
      notifications.show({ message: `Blocked ${label}` })
      return true
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
      return false
    }
  }

  return { following, muted, blocking: blockUser.isPending, toggleFollow, toggleMute, block }
}
