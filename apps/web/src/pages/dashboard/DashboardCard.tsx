import { useMemo, useState, type ReactElement } from 'react'
import { AspectRatio, Text, Tooltip } from '@mantine/core'
import { IconClock, IconFlame, IconLock, IconSparkles } from '@tabler/icons-react'
import { ApiImage } from '@/components'
import { BookmarkFavicon } from '@/components/bookmark/BookmarkCardPreview'
import { ShareBadge } from '@/components/share-badge'
import { useRecordBookmarkOpen } from '@/hooks'
import { inviteeShareAccess } from '@/lib'
import type { Bookmark, DashboardSectionKind } from '@/schemas'
import { hostname, previewSrcCandidates, type ItemMeta } from '@/services'
import styles from './DashboardCard.module.css'

type Props = {
  bookmark: Bookmark
  kind: DashboardSectionKind
  meta: ItemMeta
  viewerUserId: string | undefined
  /** Large cards also show two lines of description. */
  size: 'medium' | 'large'
}

function MetaIcon({ kind }: { kind: DashboardSectionKind }): ReactElement | null {
  if (kind === 'most_opened') return <IconFlame size={13} stroke={1.75} aria-hidden />
  if (kind === 'recently_visited' || kind === 'latest') return <IconClock size={13} stroke={1.75} aria-hidden />
  return null
}

/**
 * A dashboard bookmark card: preview, site, title and what the section adds (rank and opens,
 * when you opened it, when it was saved) in a footer. Every card in a row is the same height:
 * no tags, folders or expanding description, and the whole card is one link.
 */
export function DashboardCard({ bookmark: b, kind, meta, viewerUserId, size }: Props): ReactElement {
  const title = b.title.trim() || 'Untitled'
  const recordOpen = useRecordBookmarkOpen()
  const candidates = useMemo(() => previewSrcCandidates(b), [b])
  const [srcIndex, setSrcIndex] = useState(0)
  const previewSrc = candidates[srcIndex]
  const share = inviteeShareAccess(b.owner_user_id, b.can_write, viewerUserId)
  const showOutgoing = !share && !!b.shared_out

  return (
    <a
      href={b.url}
      target="_blank"
      rel="noreferrer"
      className={styles.card}
      onClick={() => {
        if (!b.is_vault) recordOpen(b.id)
      }}
    >
      <AspectRatio ratio={16 / 9} className={styles.preview}>
        {previewSrc ? (
          <ApiImage src={previewSrc} alt="" h="100%" fit="cover" onError={() => setSrcIndex((i) => i + 1)} />
        ) : (
          <div className={styles.previewFallback}>
            <BookmarkFavicon url={b.url} faviconUrl={b.favicon_url} isVault={b.is_vault} size={28} />
          </div>
        )}
      </AspectRatio>

      <div className={styles.body}>
        <div className={styles.site}>
          <BookmarkFavicon url={b.url} faviconUrl={b.favicon_url} isVault={b.is_vault} size={14} />
          <span className={styles.host}>{hostname(b.url)}</span>
          {b.is_vault ? (
            <Tooltip label="Stored in vault">
              <IconLock size={12} stroke={1.75} aria-label="Vault" />
            </Tooltip>
          ) : null}
        </div>

        <Text className={styles.title} lineClamp={2}>
          {title}
        </Text>

        {size === 'large' && b.description ? (
          <Text size="xs" c="dimmed" lineClamp={2} className={styles.description}>
            {b.description}
          </Text>
        ) : null}

        <div className={styles.footer}>
          {meta.rank ? <span className={styles.rank}>#{meta.rank}</span> : null}
          {meta.text ? (
            <span className={styles.stat}>
              <MetaIcon kind={kind} />
              {meta.text}
            </span>
          ) : null}
          {meta.fresh ? (
            <span className={styles.fresh}>
              <IconSparkles size={12} stroke={1.75} aria-hidden />
              New
            </span>
          ) : null}
          {share ? <ShareBadge access={share} ownerLabel={b.owner_display_name} /> : null}
          {showOutgoing ? <ShareBadge outgoing sharedWith={b.shared_with} /> : null}
        </div>
      </div>
    </a>
  )
}
