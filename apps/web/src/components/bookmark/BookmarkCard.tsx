import { useMemo, useState, type ReactElement } from 'react'
import { Card, Checkbox, Group, Highlight, Pill, Spoiler, Stack, Text, Tooltip, UnstyledButton } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconExternalLink } from '@tabler/icons-react'
import { Link, useNavigate } from '@tanstack/react-router'
import { SocialDrawer } from '@/components/social'
import { useOwnerActions, useSaveBookmarkCopy } from '@/hooks'
import { inviteeShareAccess } from '@/lib'
import { FOLDER_UNCATEGORIZED, previewSrcCandidates } from '@/services'
import { BlockOwnerModal } from './BlockOwnerModal'
import styles from './BookmarkCard.module.css'
import { BookmarkCardBadges } from './BookmarkCardBadges'
import { BookmarkCardMenu } from './BookmarkCardMenu'
import { BookmarkCardPreview } from './BookmarkCardPreview'
import type { BookmarkCardItem } from './bookmarkCardTypes'

export type { BookmarkCardItem } from './bookmarkCardTypes'

type Props = {
  bookmark: BookmarkCardItem
  highlightQuery?: string
  /** Current user — used to show Shared · RO/RW when item is from another owner. */
  viewerUserId?: string
  onEdit: (id: string) => void
  onDelete: (id: string) => void
  /** Library only: owners can move a bookmark into or out of a vault. */
  onMoveToVault?: (id: string) => void
  /** Toggle per-viewer flags (favorite / unread / pin). */
  onFlags?: (id: string, patch: Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>) => void
  selectMode?: boolean
  selected?: boolean
  onToggleSelect?: (id: string) => void
}

export function BookmarkCard({
  bookmark: b,
  highlightQuery,
  viewerUserId,
  onEdit,
  onDelete,
  onMoveToVault,
  onFlags,
  selectMode = false,
  selected = false,
  onToggleSelect,
}: Props): ReactElement {
  const title = b.title.trim() || 'Untitled'
  const canWrite = b.can_write !== false
  const selectable = selectMode && canWrite && !!onToggleSelect
  const share = inviteeShareAccess(b.owner_user_id, b.can_write, viewerUserId)
  const showOutgoing = !share && !!b.shared_out
  const socialEnabled = !!share || showOutgoing
  const canSaveCopy = !!viewerUserId && !!b.owner_user_id && b.owner_user_id !== viewerUserId && !b.is_vault
  const canFollowOwner = !!viewerUserId && !!b.owner_user_id && b.owner_user_id !== viewerUserId
  const saveCopy = useSaveBookmarkCopy()
  const owner = useOwnerActions(b.owner_user_id, b.owner_display_name, canFollowOwner)
  const navigate = useNavigate()
  const [socialOpen, setSocialOpen] = useState(false)
  const [blockOpen, setBlockOpen] = useState(false)
  const candidates = useMemo(() => previewSrcCandidates(b), [b])

  async function onSaveCopy(): Promise<void> {
    try {
      await saveCopy.mutateAsync(b.id)
      notifications.show({ message: 'Saved to your library (Unsorted)' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onConfirmBlock(): Promise<void> {
    if (await owner.block()) setBlockOpen(false)
  }

  const previewKey = `${b.id}:${String(b.has_preview)}:${b.preview_image_url ?? ''}:${b.updated_at}`
  const [srcIndex, setSrcIndex] = useState(0)
  const [srcForKey, setSrcForKey] = useState(previewKey)
  if (srcForKey !== previewKey) {
    setSrcForKey(previewKey)
    setSrcIndex(0)
  }
  const previewSrc = candidates[srcIndex]

  const cats = b.categories || []
  const q = highlightQuery?.trim()
  const pendingDelete = !!b.deleted_at
  const cardClass = [
    styles.card,
    selected ? styles.selected : '',
    selectMode && !canWrite ? styles.selectDisabled : '',
    pendingDelete ? styles.pendingDelete : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <>
      <Card withBorder padding="sm" radius="md" h="100%" className={cardClass}>
        {selectable ? (
          <Checkbox
            className={styles.selectCheck}
            checked={selected}
            onChange={() => onToggleSelect?.(b.id)}
            aria-label={`Select ${title}`}
            onClick={(e) => e.stopPropagation()}
          />
        ) : null}
        <Card.Section>
          <BookmarkCardPreview
            url={b.url}
            title={title}
            previewSrc={previewSrc}
            faviconUrl={b.favicon_url}
            isVault={b.is_vault}
            onPreviewError={() => setSrcIndex((i) => i + 1)}
          />
        </Card.Section>

        <Stack gap={8} mt="md" className={styles.body}>
          <Group justify="space-between" align="flex-start" wrap="nowrap" gap="xs">
            <UnstyledButton
              component="a"
              href={b.url}
              target="_blank"
              rel="noreferrer"
              className={styles.titleLink}
              onClick={() => {
                if (b.is_unread && onFlags) onFlags(b.id, { is_unread: false })
              }}
            >
              <Group gap={6} wrap="nowrap" align="flex-start">
                <IconExternalLink size={14} stroke={1.5} className={styles.externalIcon} />
                {q ? (
                  <Highlight highlight={q} fw={600} lineClamp={2} className={styles.titleText}>
                    {title}
                  </Highlight>
                ) : (
                  <Text fw={600} lineClamp={2} className={styles.titleText}>
                    {title}
                  </Text>
                )}
              </Group>
            </UnstyledButton>
            <BookmarkCardMenu
              url={b.url}
              canWrite={canWrite}
              canSaveCopy={canSaveCopy}
              canFollowOwner={canFollowOwner}
              followingOwner={owner.following}
              mutedOwner={owner.muted}
              ownerLabel={b.owner_display_name}
              socialEnabled={socialEnabled}
              isFavorite={b.is_favorite}
              isUnread={b.is_unread}
              isPinned={b.is_pinned}
              onFlags={onFlags ? (patch) => onFlags(b.id, patch) : undefined}
              onSaveCopy={() => void onSaveCopy()}
              onViewOwnerProfile={() => void navigate({ to: '/u/$id', params: { id: b.owner_user_id! } })}
              onToggleFollowOwner={() => void owner.toggleFollow()}
              onToggleMuteOwner={() => void owner.toggleMute()}
              onBlockOwner={() => setBlockOpen(true)}
              onOpenSocial={() => setSocialOpen(true)}
              onEdit={() => onEdit(b.id)}
              onDelete={() => onDelete(b.id)}
              onMoveToVault={
                onMoveToVault && b.owner_user_id === viewerUserId && !b.deleted_at
                  ? () => onMoveToVault(b.id)
                  : undefined
              }
            />
          </Group>

          <Text size="xs" c="dimmed" lineClamp={1} title={b.url}>
            {b.url}
          </Text>

          {b.description ? (
            <Spoiler maxHeight={40} showLabel="More" hideLabel="Less">
              <Text size="sm" c="dimmed">
                {b.description}
              </Text>
            </Spoiler>
          ) : null}

          {cats.length > 0 ? (
            <Group gap={4} className={styles.categoryRow}>
              {cats.map((c) => (
                <Tooltip key={c.id} label={c.path} openDelay={400}>
                  <Link
                    to="/library"
                    search={{ category: c.id }}
                    onClick={(e) => e.stopPropagation()}
                    className={styles.categoryLink}
                  >
                    <Pill size="sm" className={styles.categoryPill}>
                      {c.path}
                    </Pill>
                  </Link>
                </Tooltip>
              ))}
            </Group>
          ) : (
            <Group gap={4} className={styles.categoryRow}>
              <Tooltip label="No category — open Unsorted in library" openDelay={400}>
                <Link
                  to="/library"
                  search={{ folder: FOLDER_UNCATEGORIZED, category: FOLDER_UNCATEGORIZED }}
                  onClick={(e) => e.stopPropagation()}
                  className={styles.categoryLink}
                >
                  <Pill size="sm" className={styles.unsortedPill}>
                    Unsorted
                  </Pill>
                </Link>
              </Tooltip>
            </Group>
          )}

          <BookmarkCardBadges bookmark={b} share={share} showOutgoing={showOutgoing} pendingDelete={pendingDelete} />
        </Stack>
      </Card>
      {socialEnabled ? (
        <SocialDrawer
          opened={socialOpen}
          onClose={() => setSocialOpen(false)}
          subjectType="bookmark"
          subjectId={b.id}
          title={title}
        />
      ) : null}
      {canFollowOwner ? (
        <BlockOwnerModal
          opened={blockOpen}
          ownerLabel={b.owner_display_name}
          loading={owner.blocking}
          onClose={() => setBlockOpen(false)}
          onConfirm={() => void onConfirmBlock()}
        />
      ) : null}
    </>
  )
}
