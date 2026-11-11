import { useMemo, useRef, useState, type CSSProperties, type ReactElement, type ReactNode } from 'react'
import {
  Alert,
  Badge,
  Box,
  Button,
  Center,
  Drawer,
  Group,
  Pagination,
  ScrollArea,
  SegmentedControl,
  Stack,
  Text,
  Title,
} from '@mantine/core'
import {
  IconAlertCircle,
  IconArrowMerge,
  IconCopyPlus,
  IconFolder,
  IconPlayerSkipForward,
  IconRefresh,
  type Icon,
} from '@tabler/icons-react'
import { BookmarkFavicon } from '@/components/bookmark/BookmarkCardPreview'
import { useDrawerSize } from '@/hooks/render'
import type { DuplicateAction, DuplicateDecisions, ImportDuplicate, ImportPreview } from '@/schemas'
import { hostname } from '@/services'
import { actionFor, changedFields, decisionsBody, tally, type DuplicateField } from './duplicateDecisions'
import classes from './ImportDuplicatesDrawer.module.css'

const PAGE = 50
const MAX_TAGS = 3

type Choice = { value: DuplicateAction; label: string; icon: Icon; done: string; hint: string }

const CHOICES: Choice[] = [
  {
    value: 'skip',
    label: 'Skip',
    icon: IconPlayerSkipForward,
    done: 'skipped',
    hint: 'Keep the bookmark you have and leave the one in the file out.',
  },
  {
    value: 'replace',
    label: 'Replace',
    icon: IconRefresh,
    done: 'replaced',
    hint: "Update your bookmark with the file's title, description, tags and folders. Its shares, pins and activity stay.",
  },
  {
    value: 'merge',
    label: 'Merge',
    icon: IconArrowMerge,
    done: 'merged',
    hint: "Keep your title and description, fill any that are empty from the file, and add the file's tags and folders to yours.",
  },
  {
    value: 'keep',
    label: 'Import anyway',
    icon: IconCopyPlus,
    done: 'imported again',
    hint: 'Add the one in the file as a second bookmark.',
  },
]

const choiceFor = (action: DuplicateAction): Choice => CHOICES.find((c) => c.value === action)!

/** Segment labels. `compact` hides the words on phones, keeping them for screen readers. */
function choiceData(compact: boolean): { value: string; label: ReactNode }[] {
  return CHOICES.map(({ value, label, icon: ChoiceIcon }) => ({
    value,
    label: (
      <Group gap={6} wrap="nowrap" justify="center">
        <ChoiceIcon size={compact ? 15 : 16} stroke={1.75} aria-hidden />
        <span className={compact ? classes.compactLabel : undefined}>{label}</span>
      </Group>
    ),
  }))
}

function ChoiceControl({
  value,
  onChange,
  compact,
  label,
}: {
  value: DuplicateAction
  onChange: (action: DuplicateAction) => void
  compact: boolean
  label: string
}): ReactElement {
  return (
    <SegmentedControl
      fullWidth
      size={compact ? 'xs' : 'sm'}
      radius="md"
      data={choiceData(compact)}
      value={value}
      onChange={(v) => onChange(v as DuplicateAction)}
      data-action={value}
      classNames={{ indicator: classes.indicator, label: classes.segmentLabel }}
      aria-label={label}
    />
  )
}

type Props = {
  opened: boolean
  preview: ImportPreview
  pending: boolean
  error: string | null
  onConfirm: (decisions: DuplicateDecisions) => void
  onClose: () => void
}

/** Drawer.Content wraps children in the theme's scroll area; this frame replaces it so the
 *  header and footer stay fixed and only the list scrolls. */
function PanelFrame({ style, children }: { style?: CSSProperties; children?: ReactNode }): ReactElement {
  return (
    <div style={style} className={classes.content}>
      {children}
    </div>
  )
}

function Stat({ label, value }: { label: string; value: number }): ReactElement {
  return (
    <div className={classes.stat}>
      <span className={classes.statValue}>{value.toLocaleString()}</span>
      <span className={classes.statLabel}>{label}</span>
    </div>
  )
}

type SideState = 'winner' | 'muted' | 'normal'
type SideLook = { state: SideState; status: string }

/** What each choice does to the two sides of a row. */
function sideLooks(action: DuplicateAction): { library: SideLook; file: SideLook } {
  switch (action) {
    case 'replace':
      return { library: { state: 'muted', status: 'Replaced' }, file: { state: 'winner', status: 'Used' } }
    case 'merge':
      return {
        library: { state: 'winner', status: 'Merged into' },
        file: { state: 'normal', status: 'Tags and folders added' },
      }
    case 'keep':
      return { library: { state: 'normal', status: 'Kept' }, file: { state: 'winner', status: 'Added as a copy' } }
    default:
      return { library: { state: 'normal', status: 'Kept' }, file: { state: 'muted', status: 'Left out' } }
  }
}

function Side({
  label,
  look,
  title,
  description,
  tags,
  folders,
}: {
  label: string
  look: SideLook
  title?: string | null
  description?: string | null
  tags: string[]
  folders: string[]
}): ReactElement {
  const extraTags = tags.length - MAX_TAGS
  return (
    <div className={classes.side} data-state={look.state}>
      <div className={classes.sideHead}>
        <span className={classes.eyebrow}>{label}</span>
        <span className={classes.status}>{look.status}</span>
      </div>
      <Text size="sm" className={classes.sideTitle} lineClamp={2} dir="auto" c={title ? undefined : 'dimmed'}>
        {title || 'No title'}
      </Text>
      {description ? (
        <Text size="xs" c="dimmed" lineClamp={1} dir="auto" mt={4}>
          {description}
        </Text>
      ) : null}
      <div className={classes.meta}>
        <IconFolder size={14} stroke={1.75} aria-hidden style={{ flex: 'none' }} />
        <Text span size="xs" truncate="end" dir="auto">
          {folders.length ? folders.join(', ') : 'Unsorted'}
        </Text>
      </div>
      {tags.length ? (
        <Group gap={4} mt="xs">
          {tags.slice(0, MAX_TAGS).map((t) => (
            <Badge key={t} size="sm" variant="light" color="gray" radius="sm" tt="none" fw={500}>
              <span dir="auto">{t}</span>
            </Badge>
          ))}
          {extraTags > 0 ? (
            <Text span size="xs" c="dimmed" title={tags.slice(MAX_TAGS).join(', ')}>
              +{extraTags}
            </Text>
          ) : null}
        </Group>
      ) : null}
    </div>
  )
}

function changeSummary(fields: DuplicateField[]): string {
  if (fields.length === 0) return 'Same as yours'
  const list = fields.length === 1 ? fields[0] : `${fields.slice(0, -1).join(', ')} and ${fields[fields.length - 1]}`
  return `Different ${list}`
}

function urlParts(url: string): { host: string; rest: string } {
  const host = hostname(url) || url
  try {
    const u = new URL(url)
    return { host, rest: `${u.pathname === '/' ? '' : u.pathname}${u.search}` }
  } catch {
    return { host, rest: '' }
  }
}

function DuplicateRow({
  duplicate,
  action,
  compact,
  onChange,
}: {
  duplicate: ImportDuplicate
  action: DuplicateAction
  compact: boolean
  onChange: (action: DuplicateAction) => void
}): ReactElement {
  const { host, rest } = urlParts(duplicate.url)
  const looks = sideLooks(action)
  const changed = changedFields(duplicate)
  return (
    <div className={classes.row} data-action={action} role="group" aria-label={duplicate.url}>
      <div className={classes.rowHeader}>
        <span className={classes.favicon}>
          <BookmarkFavicon url={duplicate.url} size={16} />
        </span>
        <div className={classes.urlBlock}>
          <Text size="sm" fw={600} truncate="end">
            {host}
          </Text>
          {rest ? (
            <Text size="xs" c="dimmed" truncate="end" title={duplicate.url}>
              {rest}
            </Text>
          ) : null}
        </div>
        <Text size="xs" c="dimmed" style={{ flex: 'none' }}>
          {changeSummary(changed)}
        </Text>
      </div>
      <div className={classes.sides}>
        <Side label="Your library" look={looks.library} {...duplicate.existing} />
        <Side label="In the file" look={looks.file} {...duplicate.incoming} />
      </div>
      <div className={classes.choices}>
        <ChoiceControl
          value={action}
          onChange={onChange}
          compact={compact}
          label={`What to do with ${duplicate.url}`}
        />
      </div>
    </div>
  )
}

export function ImportDuplicatesDrawer({ opened, preview, pending, error, onConfirm, onClose }: Props): ReactElement {
  const drawerSize = useDrawerSize('xl')
  const phone = drawerSize === '100%'
  const [all, setAll] = useState<DuplicateAction>('skip')
  const [picked, setPicked] = useState<Map<number, DuplicateAction>>(new Map())
  const [page, setPage] = useState(1)
  const viewport = useRef<HTMLDivElement>(null)

  const duplicates = preview.duplicates
  const pages = Math.ceil(duplicates.length / PAGE)
  const rows = duplicates.slice((page - 1) * PAGE, page * PAGE)
  const indexes = useMemo(() => duplicates.map((d) => d.index), [duplicates])
  const counts = tally(indexes, all, picked)
  const summary = CHOICES.filter((c) => counts[c.value] > 0)

  function goTo(next: number): void {
    setPage(next)
    viewport.current?.scrollTo({ top: 0 })
  }

  return (
    <Drawer.Root opened={opened} onClose={onClose} position="right" size={drawerSize} scrollAreaComponent={PanelFrame}>
      <Drawer.Overlay />
      <Drawer.Content aria-labelledby="import-duplicates-title">
        <header className={classes.header}>
          <Group justify="space-between" align="flex-start" wrap="nowrap">
            <Stack gap={6}>
              <Title order={3} id="import-duplicates-title">
                Review duplicates
              </Title>
              <Text size="sm" c="dimmed" maw={520}>
                {`${duplicates.length} of ${preview.total} bookmarks are already in your library.`} Choose what happens
                to them before anything is imported.
              </Text>
            </Stack>
            <Drawer.CloseButton aria-label="Close" />
          </Group>

          <div className={classes.stats} style={{ marginTop: 'var(--mantine-spacing-lg)' }}>
            <Stat label="already saved" value={duplicates.length} />
            <Stat label="new" value={preview.new} />
            <Stat label="folders reused" value={preview.folders.reused} />
          </div>

          <Stack gap="xs" mt="xl" role="group" aria-label="Apply to all">
            <span className={classes.eyebrow}>Apply to all</span>
            <ChoiceControl value={all} onChange={setAll} compact={phone} label="Apply to all" />
            <Text size="xs" c="dimmed" mt={2}>
              {choiceFor(all).hint}
            </Text>
          </Stack>
        </header>

        <ScrollArea
          className={classes.scroll}
          classNames={{ content: classes.scrollContent }}
          viewportRef={viewport}
          scrollbars="y"
          type="auto"
        >
          <Stack gap="lg" className={classes.list}>
            {rows.map((d) => (
              <DuplicateRow
                key={d.index}
                duplicate={d}
                action={actionFor(d.index, all, picked)}
                compact={phone}
                onChange={(action) => setPicked((m) => new Map(m).set(d.index, action))}
              />
            ))}
            {pages > 1 ? (
              <Center pt="xs">
                <Pagination value={page} onChange={goTo} total={pages} size="sm" radius="md" />
              </Center>
            ) : null}
          </Stack>
        </ScrollArea>

        <footer className={classes.footer}>
          <Stack gap="sm">
            {error ? (
              <Alert color="red" variant="light" icon={<IconAlertCircle size={16} />} title="Import failed">
                {error}
              </Alert>
            ) : null}
            <Group justify="space-between" gap="sm" wrap="nowrap">
              <Box miw={0}>
                <Text size="xs" c="dimmed" component="div">
                  {summary.map((c, i) => (
                    <span key={c.value} data-action={c.value}>
                      {i > 0 ? ' · ' : ''}
                      <span className={classes.dot} />
                      {counts[c.value]} {c.done}
                    </span>
                  ))}
                </Text>
                {preview.new > 0 ? (
                  <Text size="xs" c="dimmed" mt={2}>
                    {preview.new} new bookmark{preview.new === 1 ? '' : 's'} will be added.
                  </Text>
                ) : null}
              </Box>
              <Group gap="sm" wrap="nowrap">
                <Button variant="default" onClick={onClose} disabled={pending}>
                  Cancel
                </Button>
                <Button onClick={() => onConfirm(decisionsBody(all, picked))} loading={pending}>
                  Import
                </Button>
              </Group>
            </Group>
          </Stack>
        </footer>
      </Drawer.Content>
    </Drawer.Root>
  )
}
