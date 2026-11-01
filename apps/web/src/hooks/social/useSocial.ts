import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { SocialSubjectType, SocialSummary } from '@/schemas'

type Envelope = { social: SocialSummary }

function socialKey(subjectType: SocialSubjectType, subjectId: string) {
  return ['social', subjectType, subjectId] as const
}

export function useSocial(
  subjectType: SocialSubjectType,
  subjectId: string,
  enabled = true
): UseQueryResult<Envelope> {
  return useQuery({
    queryKey: socialKey(subjectType, subjectId),
    queryFn: () => api.getSocial(subjectType, subjectId),
    enabled: enabled && !!subjectId,
  })
}

export function useToggleSocialLike(
  subjectType: SocialSubjectType,
  subjectId: string
): UseMutationResult<Envelope, Error, boolean> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (liked: boolean) =>
      liked ? api.likeSocial(subjectType, subjectId) : api.unlikeSocial(subjectType, subjectId),
    onSuccess: (res) => {
      qc.setQueryData(socialKey(subjectType, subjectId), res)
      void qc.invalidateQueries({ queryKey: ['notifications'] })
      void qc.invalidateQueries({ queryKey: ['notifications', 'unread'] })
    },
  })
}

export function useAddSocialComment(
  subjectType: SocialSubjectType,
  subjectId: string
): UseMutationResult<Envelope, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: string) => api.commentSocial(subjectType, subjectId, body),
    onSuccess: (res) => {
      qc.setQueryData(socialKey(subjectType, subjectId), res)
      void qc.invalidateQueries({ queryKey: ['notifications'] })
      void qc.invalidateQueries({ queryKey: ['notifications', 'unread'] })
    },
  })
}

export function useDeleteSocialComment(
  subjectType: SocialSubjectType,
  subjectId: string
): UseMutationResult<{ ok: boolean }, Error, string> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => api.deleteSocialComment(id),
    onSuccess: () => {
      void qc.invalidateQueries({ queryKey: socialKey(subjectType, subjectId) })
    },
  })
}

export function useSetSocialRating(
  subjectType: SocialSubjectType,
  subjectId: string
): UseMutationResult<Envelope, Error, number | null> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (rating: number | null) =>
      rating == null
        ? api.clearSocialRating(subjectType, subjectId)
        : api.rateSocial(subjectType, subjectId, rating),
    onSuccess: (res) => {
      qc.setQueryData(socialKey(subjectType, subjectId), res)
      void qc.invalidateQueries({ queryKey: ['notifications'] })
      void qc.invalidateQueries({ queryKey: ['notifications', 'unread'] })
    },
  })
}
