import { type ReactElement } from 'react'
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
import { ActionIcon, Avatar, Button, Group, Text } from '@mantine/core'
import { IconGripVertical, IconPlus, IconX } from '@tabler/icons-react'
import type { Bookmark } from '@/schemas'
import { googleFavicon, type DashboardDraftSection } from '@/services'
import editor from './DashboardBoardEditor.module.css'
import { listKeyboardCoordinates } from './listKeyboardCoordinates'

const KEYBOARD_HELP =
  'To move it with the keyboard, press Space, use the arrow keys, then press Space again to drop it or Escape to cancel.'

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

export function PinnedBody({
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
