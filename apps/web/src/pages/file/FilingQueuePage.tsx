import { useEffect, useEffectEvent, useMemo, useRef, useState, type ReactElement, type TouchEvent } from 'react'
import {
  Anchor,
  Badge,
  Box,
  Button,
  Center,
  Group,
  Kbd,
  Loader,
  Paper,
  Stack,
  Switch,
  TagsInput,
  Text,
  TextInput,
  ThemeIcon,
  Title,
  UnstyledButton,
} from '@mantine/core'
import { useMediaQuery } from '@mantine/hooks'
import { notifications } from '@mantine/notifications'
import { IconFolderCheck, IconInbox, IconPlayerSkipForward, IconTrash } from '@tabler/icons-react'
import { useNavigate } from '@tanstack/react-router'
import { EmptyState, QueryErrorAlert } from '@/components'
import { BookmarkCardPreview } from '@/components/bookmark/BookmarkCardPreview'
import { BookmarkCategoriesField } from '@/components/bookmark/BookmarkCategoriesField'
import { MASCOT } from '@/components/mascot'
import { useCategories, useDeleteBookmark, useFilingSuggestions, useUnsortedQueue, useUpdateBookmark } from '@/hooks'
import { categoryPath } from '@/lib'
import type { Bookmark, Category } from '@/schemas'
import { FOLDER_UNCATEGORIZED, hostname, previewSrcCandidates } from '@/services'
import { currentCard, leftToFile, queueKey, toggle } from './filingQueue'
import classes from './FilingQueuePage.module.css'

/** How far a swipe must travel, in pixels, to count. */
const SWIPE = 80

type Draft = { title: string; folders: string[]; tags: string[]; favorite: boolean }

function keyTarget(target: EventTarget | null): 'page' | 'title' | 'field' {
  if (!(target instanceof HTMLElement)) return 'page'
  if (target.dataset.filingTitle !== undefined) return 'title'
  if (target.closest('input, textarea, select, [contenteditable="true"], [role="combobox"]')) return 'field'
  return 'page'
}

function FilingCard({
  bookmark,
  categories,
  busy,
  onFile,
  onSkip,
  onDelete,
}: {
  bookmark: Bookmark
  categories: Category[]
  busy: boolean
  onFile: (draft: Draft) => void
  onSkip: () => void
  onDelete: () => void
}): ReactElement {
  const wide = useMediaQuery('(min-width: 48em)', true)
  const [draft, setDraft] = useState<Draft>({
    title: bookmark.title ?? '',
    folders: [],
    tags: bookmark.tags ?? [],
    favorite: bookmark.is_favorite ?? false,
  })
  const suggestions = useFilingSuggestions(bookmark.id)
  const byId = useMemo(() => new Map(categories.map((c) => [c.id, c])), [categories])
  const folderSuggestions = (suggestions.data?.folders ?? [])
    .map((f) => ({ ...f, cat: byId.get(f.id) }))
    .filter((f): f is typeof f & { cat: Category } => !!f.cat)
  const tagSuggestions = (suggestions.data?.tags ?? []).filter((t) => !draft.tags.includes(t.name))
  const candidates = useMemo(() => previewSrcCandidates(bookmark), [bookmark])
  const [srcIndex, setSrcIndex] = useState(0)
  const canFile = draft.folders.length > 0 && !busy

  // The listener is added once per card; the event reads the latest draft.
  const onKey = useEffectEvent((e: KeyboardEvent) => {
    const action = queueKey(e, keyTarget(e.target))
    if (!action || busy) return
    if (action.kind === 'file') {
      if (!canFile) return
      e.preventDefault()
      onFile(draft)
    } else if (action.kind === 'skip') {
      e.preventDefault()
      onSkip()
    } else if (action.kind === 'delete') {
      e.preventDefault()
      onDelete()
    } else {
      const pick = folderSuggestions[action.index]
      if (!pick) return
      e.preventDefault()
      setDraft((d) => ({ ...d, folders: toggle(d.folders, pick.id) }))
    }
  })

  useEffect(() => {
    const listener = (e: KeyboardEvent): void => onKey(e)
    window.addEventListener('keydown', listener)
    return () => window.removeEventListener('keydown', listener)
  }, [])

  const touchStart = useRef<{ x: number; y: number } | null>(null)
  function onTouchStart(e: TouchEvent): void {
    const t = e.touches[0]
    touchStart.current = t ? { x: t.clientX, y: t.clientY } : null
  }
  function onTouchEnd(e: TouchEvent): void {
    const start = touchStart.current
    const t = e.changedTouches[0]
    touchStart.current = null
    if (!start || !t || busy) return
    const dx = t.clientX - start.x
    const dy = t.clientY - start.y
    // Mostly sideways, and far enough; vertical drags scroll the page.
    if (Math.abs(dx) < SWIPE || Math.abs(dx) < Math.abs(dy) * 1.5) return
    if (dx < 0) onSkip()
    else if (canFile) onFile(draft)
  }

  const host = hostname(bookmark.url) || bookmark.url
  return (
    <Paper withBorder radius="lg" className={classes.card} onTouchStart={onTouchStart} onTouchEnd={onTouchEnd}>
      <Stack gap="lg" className={classes.body}>
        <div className={classes.summary}>
          <div className={classes.thumb}>
            <BookmarkCardPreview
              url={bookmark.url}
              title={bookmark.title || host}
              previewSrc={candidates[srcIndex]}
              faviconUrl={bookmark.favicon_url}
              onPreviewError={() => setSrcIndex((i) => i + 1)}
            />
          </div>
          <Stack gap={4} miw={0}>
            <Text fw={600} truncate="end">
              {host}
            </Text>
            <Anchor href={bookmark.url} target="_blank" rel="noreferrer" size="xs" c="dimmed" className={classes.url}>
              {bookmark.url}
            </Anchor>
            {bookmark.description ? (
              <Text size="sm" c="dimmed" lineClamp={2} dir="auto">
                {bookmark.description}
              </Text>
            ) : null}
          </Stack>
        </div>

        <TextInput
          label="Title"
          value={draft.title}
          placeholder={host}
          onChange={(e) => {
            const title = e.currentTarget.value
            setDraft((d) => ({ ...d, title }))
          }}
          data-filing-title=""
          dir="auto"
        />

        <Stack gap="xs">
          {folderSuggestions.length ? (
            <Group gap={6} aria-label="Folders you use on this site" role="group">
              {folderSuggestions.map((f, i) => {
                const on = draft.folders.includes(f.id)
                return (
                  <UnstyledButton
                    key={f.id}
                    className={classes.suggestion}
                    data-active={on || undefined}
                    aria-pressed={on}
                    onClick={() => setDraft((d) => ({ ...d, folders: toggle(d.folders, f.id) }))}
                  >
                    {wide ? <Kbd size="xs">{i + 1}</Kbd> : null}
                    <span>{categoryPath(f.cat, byId)}</span>
                  </UnstyledButton>
                )
              })}
            </Group>
          ) : null}
          <BookmarkCategoriesField
            categories={categories}
            value={draft.folders}
            onChange={(folders) => setDraft((d) => ({ ...d, folders }))}
            canCreate
          />
        </Stack>

        <Stack gap="xs">
          <TagsInput label="Tags" value={draft.tags} onChange={(tags) => setDraft((d) => ({ ...d, tags }))} clearable />
          {tagSuggestions.length ? (
            <Group gap={6} aria-label="Tags you use on this site" role="group">
              {tagSuggestions.map((t) => (
                <UnstyledButton
                  key={t.name}
                  className={classes.suggestion}
                  onClick={() => setDraft((d) => ({ ...d, tags: [...d.tags, t.name] }))}
                >
                  <span dir="auto">+ {t.name}</span>
                </UnstyledButton>
              ))}
            </Group>
          ) : null}
        </Stack>

        <Switch
          label="Favourite"
          checked={draft.favorite}
          onChange={(e) => {
            const favorite = e.currentTarget.checked
            setDraft((d) => ({ ...d, favorite }))
          }}
        />
      </Stack>

      <Group justify="space-between" gap="sm" className={classes.footer} wrap="nowrap">
        <Button
          variant="subtle"
          color="red"
          leftSection={<IconTrash size={16} stroke={1.75} />}
          onClick={onDelete}
          disabled={busy}
        >
          Delete
        </Button>
        <Group gap="sm" wrap="nowrap">
          <Button
            variant="default"
            leftSection={<IconPlayerSkipForward size={16} stroke={1.75} />}
            onClick={onSkip}
            disabled={busy}
          >
            Skip
          </Button>
          <Button
            leftSection={<IconFolderCheck size={16} stroke={1.75} />}
            onClick={() => onFile(draft)}
            disabled={!canFile}
            loading={busy}
          >
            File
          </Button>
        </Group>
      </Group>
    </Paper>
  )
}

export function FilingQueuePage(): ReactElement {
  const nav = useNavigate()
  const wide = useMediaQuery('(min-width: 48em)', true)
  const queue = useUnsortedQueue()
  const categories = useCategories()
  const update = useUpdateBookmark()
  const remove = useDeleteBookmark()
  const [handled, setHandled] = useState<ReadonlySet<string>>(new Set())

  const items = useMemo(() => queue.data?.pages.flatMap((p) => p.items) ?? [], [queue.data])
  const total = queue.data?.pages[0]?.total ?? 0
  const card = currentCard(items, handled)
  const left = leftToFile(total, items, handled)
  const writable = useMemo(() => (categories.data ?? []).filter((c) => !c.is_vault && !c.deleted_at), [categories.data])

  // Every loaded bookmark handled but more on the server: load the next page.
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = queue
  useEffect(() => {
    if (!card && hasNextPage && !isFetchingNextPage) void fetchNextPage()
  }, [card, hasNextPage, isFetchingNextPage, fetchNextPage])

  function done(id: string): void {
    setHandled((s) => new Set(s).add(id))
  }

  function file(b: Bookmark, d: Draft): void {
    update.mutate(
      {
        id: b.id,
        body: {
          ...(d.title.trim() && d.title.trim() !== b.title ? { title: d.title.trim() } : {}),
          category_ids: d.folders,
          tags: d.tags,
          is_favorite: d.favorite,
        },
      },
      {
        onSuccess: () => done(b.id),
        onError: (e) => notifications.show({ color: 'red', title: 'Could not file it', message: String(e) }),
      }
    )
  }

  function del(b: Bookmark): void {
    remove.mutate(b.id, {
      onSuccess: () => {
        done(b.id)
        notifications.show({ message: 'Moved to Deleted items' })
      },
      onError: (e) => notifications.show({ color: 'red', title: 'Could not delete it', message: String(e) }),
    })
  }

  let body: ReactElement
  if (queue.isError) {
    body = <QueryErrorAlert error={queue.error} onRetry={() => void queue.refetch()} />
  } else if (queue.isPending || categories.isPending || (!card && isFetchingNextPage)) {
    body = (
      <Center py="xl">
        <Loader />
      </Center>
    )
  } else if (!card) {
    body = (
      <EmptyState
        illustration={{ src: MASCOT.approves }}
        title={handled.size ? 'All done' : 'Nothing to file'}
        description={
          handled.size
            ? 'Every bookmark in Unsorted has been filed or skipped.'
            : 'Bookmarks saved without a folder land in Unsorted. They will wait for you here.'
        }
        action={{ label: 'Back to the library', onClick: () => void nav({ to: '/library' }) }}
      />
    )
  } else {
    body = (
      <FilingCard
        key={card.id}
        bookmark={card}
        categories={writable}
        busy={update.isPending || remove.isPending}
        onFile={(d) => file(card, d)}
        onSkip={() => done(card.id)}
        onDelete={() => del(card)}
      />
    )
  }

  return (
    <Stack gap="xl" className={classes.page}>
      <Group justify="space-between" align="flex-end" gap="md">
        <Group gap="md">
          <ThemeIcon size={40} radius="md" variant="light" color="violet">
            <IconInbox size={22} stroke={1.5} />
          </ThemeIcon>
          <Stack gap={2}>
            <Title order={2}>File Unsorted</Title>
            <Text size="sm" c="dimmed">
              <Anchor
                component="button"
                type="button"
                size="sm"
                c="dimmed"
                onClick={() =>
                  void nav({ to: '/library', search: { folder: FOLDER_UNCATEGORIZED, category: FOLDER_UNCATEGORIZED } })
                }
              >
                Unsorted
              </Anchor>
              {card ? ` · ${left} left` : null}
            </Text>
          </Stack>
        </Group>
        {card && wide ? (
          <Text size="xs" c="dimmed" className={classes.keys}>
            <Kbd size="xs">1</Kbd>–<Kbd size="xs">5</Kbd> folder · <Kbd size="xs">Enter</Kbd> file ·{' '}
            <Kbd size="xs">S</Kbd> skip · <Kbd size="xs">D</Kbd> delete
          </Text>
        ) : null}
        {card && !wide ? (
          <Badge variant="light" color="gray" radius="sm" tt="none" fw={500}>
            Swipe left to skip, right to file
          </Badge>
        ) : null}
      </Group>
      <Box>{body}</Box>
    </Stack>
  )
}
