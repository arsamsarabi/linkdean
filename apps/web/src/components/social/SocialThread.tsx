import { useState, type ReactElement } from 'react'
import { ActionIcon, Button, Group, Rating, Skeleton, Stack, Text, Textarea, UnstyledButton } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconFlag, IconHeart, IconHeartFilled, IconTrash } from '@tabler/icons-react'
import { ReportDrawer } from '@/components/abuse'
import { ConfirmModal } from '@/components/confirm-modal/ConfirmModal'
import { QueryErrorAlert } from '@/components/query-error-alert'
import {
  useAddSocialComment,
  useDeleteSocialComment,
  useMe,
  useSetSocialRating,
  useSocial,
  useToggleSocialLike,
} from '@/hooks'
import type { SocialSubjectType } from '@/schemas'

type Props = {
  subjectType: SocialSubjectType
  subjectId: string
  /** Load only while shown. */
  enabled: boolean
}

/** Likes, rating and comments for a shared item. Shown in a drawer or a bookmark's Discussion tab. */
export function SocialThread({ subjectType, subjectId, enabled }: Props): ReactElement {
  const social = useSocial(subjectType, subjectId, enabled)
  const toggleLike = useToggleSocialLike(subjectType, subjectId)
  const addComment = useAddSocialComment(subjectType, subjectId)
  const deleteComment = useDeleteSocialComment(subjectType, subjectId)
  const setRating = useSetSocialRating(subjectType, subjectId)
  const [draft, setDraft] = useState('')
  const [deletingId, setDeletingId] = useState<string | null>(null)
  const [reportingId, setReportingId] = useState<string | null>(null)
  const meId = useMe().data?.user.id

  const s = social.data?.social

  return (
    <>
      <Stack gap="lg">
        {social.isError ? (
          <QueryErrorAlert
            error={social.error}
            onRetry={() => void social.refetch()}
            title="Could not load likes & comments"
          />
        ) : null}
        {social.isLoading && !s ? (
          <Stack gap="sm">
            <Skeleton height={28} />
            <Skeleton height={80} />
            <Skeleton height={120} />
          </Stack>
        ) : null}

        {s ? (
          <>
            <Group gap="md" wrap="wrap">
              <UnstyledButton
                aria-label={s.liked_by_me ? 'Unlike' : 'Like'}
                disabled={toggleLike.isPending}
                onClick={() => {
                  toggleLike.mutate(!s.liked_by_me, {
                    onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                  })
                }}
              >
                <Group gap={6}>
                  {s.liked_by_me ? (
                    <IconHeartFilled size={20} stroke={1.5} color="var(--mantine-color-pink-6)" />
                  ) : (
                    <IconHeart size={20} stroke={1.5} />
                  )}
                  <Text size="sm" fw={500}>
                    {s.likes_count}
                  </Text>
                </Group>
              </UnstyledButton>

              <Group gap={6}>
                <Text size="sm" c="dimmed">
                  Rate
                </Text>
                <Rating
                  value={s.my_rating ?? 0}
                  onChange={(v) => {
                    setRating.mutate(v === 0 ? null : v, {
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    })
                  }}
                />
                {s.rating_count > 0 && s.rating_avg != null ? (
                  <Text size="xs" c="dimmed">
                    avg {s.rating_avg.toFixed(1)} ({s.rating_count})
                  </Text>
                ) : null}
              </Group>
            </Group>

            <Stack gap="sm">
              <Text fw={500} size="sm">
                Comments ({s.comments_count})
              </Text>
              {s.comments.length === 0 ? (
                <Text size="sm" c="dimmed">
                  No comments yet.
                </Text>
              ) : null}
              {s.comments.map((c) => (
                <Stack
                  key={c.id}
                  gap={4}
                  p="sm"
                  style={{ border: '1px solid var(--mantine-color-default-border)', borderRadius: 8 }}
                >
                  <Group justify="space-between" wrap="nowrap" gap="xs">
                    <Text size="xs" fw={600}>
                      {c.author_display_name}
                    </Text>
                    <Group gap={4} wrap="nowrap">
                      {meId && c.user_id !== meId ? (
                        <ActionIcon
                          size="sm"
                          variant="subtle"
                          color="gray"
                          aria-label="Report comment"
                          onClick={() => setReportingId(c.id)}
                        >
                          <IconFlag size={14} stroke={1.5} />
                        </ActionIcon>
                      ) : null}
                      {c.can_delete ? (
                        <ActionIcon
                          size="sm"
                          variant="subtle"
                          color="red"
                          aria-label="Delete comment"
                          loading={deleteComment.isPending && deleteComment.variables === c.id}
                          onClick={() => setDeletingId(c.id)}
                        >
                          <IconTrash size={14} stroke={1.5} />
                        </ActionIcon>
                      ) : null}
                    </Group>
                  </Group>
                  <Text size="sm" style={{ whiteSpace: 'pre-wrap' }}>
                    {c.body}
                  </Text>
                  <Text size="xs" c="dimmed">
                    {new Date(c.created_at).toLocaleString()}
                  </Text>
                </Stack>
              ))}

              <Textarea
                placeholder="Write a comment…"
                minRows={2}
                maxRows={6}
                autosize
                value={draft}
                onChange={(e) => setDraft(e.currentTarget.value)}
              />
              <Group justify="flex-end">
                <Button
                  size="compact-sm"
                  loading={addComment.isPending}
                  disabled={!draft.trim()}
                  onClick={() => {
                    const body = draft.trim()
                    if (!body) return
                    addComment.mutate(body, {
                      onSuccess: () => setDraft(''),
                      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
                    })
                  }}
                >
                  Comment
                </Button>
              </Group>
            </Stack>
          </>
        ) : null}
      </Stack>
      <ReportDrawer
        opened={!!reportingId}
        onClose={() => setReportingId(null)}
        targetType="comment"
        targetId={reportingId ?? ''}
        subject="comment"
      />
      <ConfirmModal
        opened={!!deletingId}
        onClose={() => setDeletingId(null)}
        onConfirm={() => {
          if (!deletingId) return
          deleteComment.mutate(deletingId, {
            onError: (err) => notifications.show({ color: 'red', message: String(err) }),
          })
          setDeletingId(null)
        }}
        title="Delete comment?"
        body="The comment is removed for everyone. It cannot be restored."
        confirmLabel="Delete"
      />
    </>
  )
}
