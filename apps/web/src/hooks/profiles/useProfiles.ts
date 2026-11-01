import { keepPreviousData, useQuery, type UseQueryResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { Profile, ProfileBookmarksPage, ProfileSummary } from '@/schemas'

export const PROFILE_PAGE_SIZE = 24

export function useProfile(userId: string, enabled = true): UseQueryResult<Profile> {
  return useQuery({
    queryKey: ['profiles', userId],
    queryFn: () => api.userProfile(userId),
    enabled: enabled && !!userId,
    retry: false,
  })
}

export function useProfileBookmarks(userId: string, page: number): UseQueryResult<ProfileBookmarksPage> {
  return useQuery({
    queryKey: ['profiles', userId, 'bookmarks', page],
    queryFn: () => api.userBookmarks(userId, PROFILE_PAGE_SIZE, (page - 1) * PROFILE_PAGE_SIZE),
    placeholderData: keepPreviousData,
    retry: false,
  })
}

export function useProfileFollows(
  userId: string,
  kind: 'followers' | 'following',
  enabled: boolean
): UseQueryResult<{ users: ProfileSummary[] }> {
  return useQuery({
    queryKey: ['profiles', userId, kind],
    queryFn: () => (kind === 'followers' ? api.userFollowers(userId) : api.userFollowing(userId)),
    enabled,
    retry: false,
  })
}
