import { useState, type ReactElement } from 'react'
import {
  closestCorners,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  pointerWithin,
  useSensor,
  useSensors,
  type Announcements,
  type CollisionDetection,
  type DragEndEvent,
} from '@dnd-kit/core'
import { rectSortingStrategy, SortableContext, useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import {
  ActionIcon,
  Avatar,
  Badge,
  Button,
  Group,
  Menu,
  SegmentedControl,
  Stack,
  Text,
  TextInput,
  Tooltip,
} from '@mantine/core'
import { IconAdjustments, IconChevronDown, IconGripVertical, IconPlus, IconTrash, IconX } from '@tabler/icons-react'
import type { Bookmark, DashboardSectionKind, DashboardSectionWidth } from '@/schemas'
import {
  describeSectionSource,
  googleFavicon,
  sectionFillsItself,
  type DashboardDraft,
  type DashboardDraftSection,
} from '@/services'
import editor from './DashboardBoardEditor.module.css'
import grid from './DashboardGrid.module.css'
import panel from './DashboardSectionPanel.module.css'
import { listKeyboardCoordinates } from './listKeyboardCoordinates'
import { kindAccent, SECTION_KIND_LOOK } from './sectionKinds'
import { WIDTH_OPTIONS } from './sectionOptions'
import { SectionSettingsDrawer, type SectionPatch } from './SectionSettingsDrawer'

const iconSm = { size: 16, stroke: 1.5 } as const

const ADD_HINTS: Record<DashboardSectionKind, string> = {
  pinned: 'Bookmarks you pick',
  latest: 'Newest bookmarks that match',
  most_opened: 'The ones you open most',
  recently_visited: 'The ones you opened last',
}

const KEYBOARD_HELP =
  'To move it with the keyboard, press Space, use the arrow keys, then press Space again to drop it or Escape to cancel.'

type Props = {
  draft: DashboardDraft
  bookmarkOptions: { value: string; label: string }[]
  bookmarkById: Map<string, Bookmark>
  folderOptions: { value: string; label: string }[]
  tagOptions: string[]
  folderName: (id: string) => string | undefined
  /** False on a shared dashboard or one someone else owns: only pinned sections can be added. */
  canAddSelfFilling: boolean
  onUpdateTitle: (idx: number, title: string) => void
  onUpdateBookmarks: (idx: number, bookmarkIds: string[]) => void
  onUpdateSection: (idx: number, patch: SectionPatch) => void
  onMoveSection: (from: number, to: number) => void
  onMovePin: (idx: number, from: number, to: number) => void
  onRemoveSection: (idx: number) => void
  /** Adds a section and returns its key. */
  onAddSection: (kind: DashboardSectionKind) => string
}

function useBoardSensors(ids: readonly string[]): ReturnType<typeof useSensors> {
  return useSensors(
    // A short move before a drag starts, so clicks on buttons and inputs still work.
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor, { coordinateGetter: listKeyboardCoordinates(ids) })
  )
}

/**
 * Drop where the pointer is. Sections differ in size, so the dragged card's center is a poor guide;
 * the keyboard has no pointer and lines corners up as it moves, so it falls back to the nearest corners.
 */
const dropAtPointer: CollisionDetection = (args) => {
  const hits = pointerWithin(args)
  return hits.length > 0 ? hits : closestCorners(args)
}

/** Screen reader messages that name sections and bookmarks instead of their ids. */
function announcements(name: (id: string) => string, position: (id: string) => number, total: number): Announcements {
  const at = (id: string): string => `position ${position(id) + 1} of ${total}`
  return {
    onDragStart: ({ active }) => `Picked up ${name(String(active.id))}, ${at(String(active.id))}.`,
    onDragOver: ({ active, over }) =>
      over ? `${name(String(active.id))} is now at ${at(String(over.id))}.` : undefined,
    onDragEnd: ({ active, over }) =>
      over ? `Dropped ${name(String(active.id))} at ${at(String(over.id))}.` : `Dropped ${name(String(active.id))}.`,
    onDragCancel: ({ active }) => `Moving ${name(String(active.id))} was cancelled.`,
  }
}

/** Calls `move(from, to)` for a finished drag inside one sortable list of ids. */
function onSortEnd(ids: string[], move: (from: number, to: number) => void) {
  return ({ active, over }: DragEndEvent): void => {
    if (!over || active.id === over.id) return
    const from = ids.indexOf(String(active.id))
    const to = ids.indexOf(String(over.id))
    if (from >= 0 && to >= 0) move(from, to)
  }
}

function SortablePin({ bookmark, onRemove }: { bookmark: Bookmark; onRemove: () => void }): ReactElement {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({
    id: bookmark.id,
  })
  const title = bookmark.title || bookmark.url
  return (
    <div
      ref={setNodeRef}
      className={isDragging ? editor.dragging : undefined}
      style={{ transform: CSS.Transform.toString(transform), transition }}
    >
      <div className={editor.pin}>
        <ActionIcon
          ref={setActivatorNodeRef}
          className={editor.handle}
          variant="subtle"
          color="gray"
          size="sm"
          aria-label={`Move bookmark “${title}”`}
          {...attributes}
          {...listeners}
        >
          <IconGripVertical size={14} stroke={1.6} />
        </ActionIcon>
        <Avatar
          src={bookmark.favicon_url || (bookmark.is_vault ? undefined : googleFavicon(bookmark.url))}
          size={20}
          radius="sm"
          alt=""
        >
          {title.charAt(0).toUpperCase()}
        </Avatar>
        <span className={editor.pinTitle} title={title}>
          {title}
        </span>
        <ActionIcon variant="subtle" color="gray" size="sm" aria-label={`Unpin “${title}”`} onClick={onRemove}>
          <IconX size={12} stroke={1.8} />
        </ActionIcon>
      </div>
    </div>
  )
}

function PinnedBody({
  section,
  bookmarkById,
  onMove,
  onUpdateBookmarks,
  onOpenSettings,
}: {
  section: DashboardDraftSection
  bookmarkById: Map<string, Bookmark>
  onMove: (from: number, to: number) => void
  onUpdateBookmarks: (ids: string[]) => void
  onOpenSettings: () => void
}): ReactElement {
  const pins = section.bookmark_ids.flatMap((id) => {
    const b = bookmarkById.get(id)
    return b ? [b] : []
  })
  const ids = pins.map((b) => b.id)
  const sensors = useBoardSensors(ids)
  const name = (id: string): string => {
    const b = bookmarkById.get(id)
    return `bookmark “${b?.title || b?.url || 'Untitled'}”`
  }

  if (pins.length === 0) {
    return (
      <Group gap="xs">
        <Text size="sm" c="dimmed">
          No bookmarks yet.
        </Text>
        <Button size="compact-sm" variant="light" leftSection={<IconPlus size={14} />} onClick={onOpenSettings}>
          Pick bookmarks
        </Button>
      </Group>
    )
  }
  return (
    <DndContext
      sensors={sensors}
      collisionDetection={dropAtPointer}
      onDragEnd={onSortEnd(ids, (from, to) =>
        onMove(section.bookmark_ids.indexOf(ids[from]!), section.bookmark_ids.indexOf(ids[to]!))
      )}
      accessibility={{
        announcements: announcements(name, (id) => ids.indexOf(id), ids.length),
        screenReaderInstructions: { draggable: KEYBOARD_HELP },
      }}
    >
      <SortableContext items={ids} strategy={rectSortingStrategy}>
        <div className={editor.pins}>
          {pins.map((b) => (
            <SortablePin
              key={b.id}
              bookmark={b}
              onRemove={() => onUpdateBookmarks(section.bookmark_ids.filter((id) => id !== b.id))}
            />
          ))}
          <Button size="compact-sm" variant="subtle" leftSection={<IconPlus size={14} />} onClick={onOpenSettings}>
            Add
          </Button>
        </div>
      </SortableContext>
    </DndContext>
  )
}

function SortableSection({
  section,
  bookmarkById,
  folderName,
  onUpdateTitle,
  onUpdateBookmarks,
  onUpdateSection,
  onMovePin,
  onRemove,
  onOpenSettings,
}: {
  section: DashboardDraftSection
  bookmarkById: Map<string, Bookmark>
  folderName: (id: string) => string | undefined
  onUpdateTitle: (title: string) => void
  onUpdateBookmarks: (ids: string[]) => void
  onUpdateSection: (patch: SectionPatch) => void
  onMovePin: (from: number, to: number) => void
  onRemove: () => void
  onOpenSettings: () => void
}): ReactElement {
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({
    id: section.key,
  })
  const look = SECTION_KIND_LOOK[section.kind]
  const label = section.title.trim() || `${look.label} section`
  const caption = sectionFillsItself(section) ? (describeSectionSource(section, folderName) ?? 'Fills itself') : null

  return (
    <div
      ref={setNodeRef}
      className={`${grid.cell}${isDragging ? ` ${editor.dragging}` : ''}`}
      data-width={section.width}
      style={{ ...kindAccent(section.kind), transform: CSS.Translate.toString(transform), transition }}
    >
      <section className={`${panel.panel} ${editor.card}`} aria-label={`Section “${label}”`} data-dash-edit-section>
        <div className={panel.header}>
          <Group gap="xs" wrap="nowrap" align="center">
            <Tooltip label="Drag to move" openDelay={400}>
              <ActionIcon
                ref={setActivatorNodeRef}
                className={editor.handle}
                variant="subtle"
                color="gray"
                aria-label={`Move section “${label}”`}
                {...attributes}
                {...listeners}
              >
                <IconGripVertical {...iconSm} />
              </ActionIcon>
            </Tooltip>
            <span className={panel.chip} aria-hidden>
              {look.icon(18)}
            </span>
            <TextInput
              className={editor.titleInput}
              variant="unstyled"
              aria-label="Section title"
              placeholder={`${look.label} section`}
              value={section.title}
              onChange={(e) => onUpdateTitle(e.currentTarget.value)}
              style={{ flex: 1, minWidth: 0 }}
            />
            <Tooltip label="Section settings">
              <ActionIcon variant="subtle" aria-label={`Settings for “${label}”`} onClick={onOpenSettings}>
                <IconAdjustments {...iconSm} />
              </ActionIcon>
            </Tooltip>
            <Tooltip label="Remove section">
              <ActionIcon variant="subtle" color="red" aria-label={`Remove “${label}”`} onClick={onRemove}>
                <IconTrash {...iconSm} />
              </ActionIcon>
            </Tooltip>
          </Group>
          <Group gap="xs" mt={8} justify="space-between" wrap="wrap">
            <Group gap={6}>
              <Badge size="sm" variant="light" color={look.color} radius="sm">
                {look.label}
              </Badge>
              {caption ? (
                <Text size="xs" c="dimmed">
                  {caption}
                </Text>
              ) : null}
            </Group>
            <SegmentedControl
              size="xs"
              aria-label={`Width of “${label}”`}
              data={WIDTH_OPTIONS}
              value={section.width}
              onChange={(v) => onUpdateSection({ width: v as DashboardSectionWidth })}
              visibleFrom="sm"
            />
          </Group>
        </div>
        <div className={editor.body}>
          {sectionFillsItself(section) ? (
            <Text size="sm" c="dimmed">
              Shows up to {section.item_limit} bookmarks. Open the settings to change what fills it.
            </Text>
          ) : (
            <PinnedBody
              section={section}
              bookmarkById={bookmarkById}
              onMove={onMovePin}
              onUpdateBookmarks={onUpdateBookmarks}
              onOpenSettings={onOpenSettings}
            />
          )}
        </div>
      </section>
    </div>
  )
}

/** Edit mode: the board itself, with drag handles, widths, and a settings drawer per section. */
export function DashboardBoardEditor({
  draft,
  bookmarkOptions,
  bookmarkById,
  folderOptions,
  tagOptions,
  folderName,
  canAddSelfFilling,
  onUpdateTitle,
  onUpdateBookmarks,
  onUpdateSection,
  onMoveSection,
  onMovePin,
  onRemoveSection,
  onAddSection,
}: Props): ReactElement {
  const [settingsKey, setSettingsKey] = useState<string | null>(null)
  const keys = draft.sections.map((s) => s.key)
  const sensors = useBoardSensors(keys)
  const settingsIdx = settingsKey ? keys.indexOf(settingsKey) : -1
  const settingsSection = settingsIdx >= 0 ? draft.sections[settingsIdx]! : null
  const name = (key: string): string => {
    const s = draft.sections.find((x) => x.key === key)
    return `section “${s?.title.trim() || (s ? SECTION_KIND_LOOK[s.kind].label : 'Untitled')}”`
  }

  return (
    <Stack gap="md">
      <Text size="sm" c="dimmed">
        Drag <IconGripVertical size={14} style={{ verticalAlign: '-2px' }} /> to move sections and pinned bookmarks.{' '}
        {KEYBOARD_HELP}
      </Text>
      {bookmarkOptions.length === 0 ? (
        <Text size="sm" c="orange">
          No bookmarks in your library yet. Add some in the library, then return here to pin them.
        </Text>
      ) : null}

      <DndContext
        sensors={sensors}
        collisionDetection={dropAtPointer}
        onDragEnd={onSortEnd(keys, onMoveSection)}
        accessibility={{
          announcements: announcements(name, (k) => keys.indexOf(k), keys.length),
          screenReaderInstructions: { draggable: KEYBOARD_HELP },
        }}
      >
        <SortableContext items={keys} strategy={rectSortingStrategy}>
          <div className={grid.grid}>
            {draft.sections.map((section, idx) => (
              <SortableSection
                key={section.key}
                section={section}
                bookmarkById={bookmarkById}
                folderName={folderName}
                onUpdateTitle={(title) => onUpdateTitle(idx, title)}
                onUpdateBookmarks={(ids) => onUpdateBookmarks(idx, ids)}
                onUpdateSection={(patch) => onUpdateSection(idx, patch)}
                onMovePin={(from, to) => onMovePin(idx, from, to)}
                onRemove={() => onRemoveSection(idx)}
                onOpenSettings={() => setSettingsKey(section.key)}
              />
            ))}
            <div className={grid.cell} data-width="third">
              <div className={editor.addTile}>
                <Menu position="bottom" width={280}>
                  <Menu.Target>
                    <Button
                      type="button"
                      variant="light"
                      leftSection={<IconPlus {...iconSm} />}
                      rightSection={<IconChevronDown {...iconSm} />}
                    >
                      Add section
                    </Button>
                  </Menu.Target>
                  <Menu.Dropdown>
                    {(Object.keys(ADD_HINTS) as DashboardSectionKind[]).map((kind) => {
                      const disabled = kind !== 'pinned' && !canAddSelfFilling
                      return (
                        <Menu.Item
                          key={kind}
                          leftSection={SECTION_KIND_LOOK[kind].icon(16)}
                          disabled={disabled}
                          onClick={() => setSettingsKey(onAddSection(kind))}
                        >
                          <Text size="sm">{SECTION_KIND_LOOK[kind].label}</Text>
                          <Text size="xs" c="dimmed">
                            {disabled ? "Only on dashboards you own and haven't shared" : ADD_HINTS[kind]}
                          </Text>
                        </Menu.Item>
                      )
                    })}
                  </Menu.Dropdown>
                </Menu>
              </div>
            </div>
          </div>
        </SortableContext>
      </DndContext>

      <SectionSettingsDrawer
        section={settingsSection}
        onClose={() => setSettingsKey(null)}
        bookmarkOptions={bookmarkOptions}
        folderOptions={folderOptions}
        tagOptions={tagOptions}
        onUpdateTitle={(title) => onUpdateTitle(settingsIdx, title)}
        onUpdateBookmarks={(ids) => onUpdateBookmarks(settingsIdx, ids)}
        onUpdateSection={(patch) => onUpdateSection(settingsIdx, patch)}
      />
    </Stack>
  )
}
