import { useState, type KeyboardEvent, type ReactElement } from 'react'
import { ActionIcon, Badge, Button, Group, Menu, Stack, Text, TextInput, Tooltip } from '@mantine/core'
import { IconAdjustments, IconArrowsMove, IconChevronDown, IconPlus, IconTrash } from '@tabler/icons-react'
import type { Layout } from 'react-grid-layout'
import type { Bookmark, DashboardSectionKind, DashboardSectionLayout } from '@/schemas'
import {
  bentoLayout,
  describeSectionSource,
  keyboardStep,
  layoutByKey,
  sectionFillsItself,
  type DashboardDraft,
  type DashboardDraftSection,
} from '@/services'
import { BentoGrid } from './BentoGrid'
import editor from './DashboardBoardEditor.module.css'
import panel from './DashboardSectionPanel.module.css'
import { PinnedBody } from './PinnedBody'
import { kindAccent, SECTION_KIND_LOOK } from './sectionKinds'
import { SectionSettingsDrawer, type SectionPatch } from './SectionSettingsDrawer'

const iconSm = { size: 16, stroke: 1.5 } as const

const ADD_HINTS: Record<DashboardSectionKind, string> = {
  pinned: 'Bookmarks you pick',
  latest: 'Newest bookmarks that match',
  most_opened: 'The ones you open most',
  recently_visited: 'The ones you opened last',
}

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
  /** Every section's place and size after a drag, a resize or a keyboard move. */
  onApplyLayout: (layout: Map<string, DashboardSectionLayout>) => void
  onMovePin: (idx: number, from: number, to: number) => void
  onRemoveSection: (idx: number) => void
  /** Adds a section and returns its key. */
  onAddSection: (kind: DashboardSectionKind) => string
}

function sectionLabel(section: DashboardDraftSection): string {
  return section.title.trim() || `${SECTION_KIND_LOOK[section.kind].label} section`
}

function EditableSection({
  section,
  bookmarkById,
  folderName,
  onUpdateTitle,
  onUpdateBookmarks,
  onMovePin,
  onRemove,
  onOpenSettings,
  onKeyboardStep,
}: {
  section: DashboardDraftSection
  bookmarkById: Map<string, Bookmark>
  folderName: (id: string) => string | undefined
  onUpdateTitle: (title: string) => void
  onUpdateBookmarks: (ids: string[]) => void
  onMovePin: (from: number, to: number) => void
  onRemove: () => void
  onOpenSettings: () => void
  onKeyboardStep: (code: string, resize: boolean) => boolean
}): ReactElement {
  const look = SECTION_KIND_LOOK[section.kind]
  const label = sectionLabel(section)
  const caption = sectionFillsItself(section) ? (describeSectionSource(section, folderName) ?? 'Fills itself') : null

  const onHandleKey = (e: KeyboardEvent<HTMLButtonElement>): void => {
    if (onKeyboardStep(e.code, e.shiftKey)) e.preventDefault()
  }

  return (
    <section
      className={`${panel.panel} ${editor.card}`}
      style={kindAccent(section.kind)}
      aria-label={`Section “${label}”`}
      data-dash-edit-section
    >
      <div className={panel.header}>
        <Group gap="xs" wrap="nowrap" align="center">
          <Tooltip label="Drag to move. With the keyboard: arrows move, Shift + arrows resize" openDelay={400}>
            <ActionIcon
              className={`dash-drag-handle ${editor.handle}`}
              variant="subtle"
              color="gray"
              aria-label={`Move or resize section “${label}”`}
              aria-describedby="dash-editor-keyboard-help"
              onKeyDown={onHandleKey}
            >
              <IconArrowsMove {...iconSm} />
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
        <Group gap={6} mt={8} wrap="wrap">
          <Badge size="sm" variant="light" color={look.color} radius="sm">
            {look.label}
          </Badge>
          <Badge size="sm" variant="outline" color="gray" radius="sm" title="Columns × rows">
            {section.layout.w} × {section.layout.h}
          </Badge>
          {caption ? (
            <Text size="xs" c="dimmed" lineClamp={1}>
              {caption}
            </Text>
          ) : null}
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
  )
}

/** Edit mode: the bento board itself. Drag, resize, and a settings drawer per section. */
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
  onApplyLayout,
  onMovePin,
  onRemoveSection,
  onAddSection,
}: Props): ReactElement {
  const [settingsKey, setSettingsKey] = useState<string | null>(null)
  const [announcement, setAnnouncement] = useState('')
  const keys = draft.sections.map((s) => s.key)
  const settingsIdx = settingsKey ? keys.indexOf(settingsKey) : -1
  const settingsSection = settingsIdx >= 0 ? draft.sections[settingsIdx]! : null
  const layout = bentoLayout(draft.sections)

  const apply = (next: Layout): void => {
    const placed = layoutByKey(next)
    const changed = draft.sections.some((s) => {
      const p = placed.get(s.key)
      return p && (p.x !== s.layout.x || p.y !== s.layout.y || p.w !== s.layout.w || p.h !== s.layout.h)
    })
    if (changed) onApplyLayout(placed)
  }

  const keyboardMove = (section: DashboardDraftSection, code: string, resize: boolean): boolean => {
    const next = keyboardStep(layout, section.key, code, resize)
    if (!next) return false
    apply(next)
    const p = layoutByKey(next).get(section.key)!
    setAnnouncement(
      resize
        ? `“${sectionLabel(section)}” is ${p.w} columns wide and ${p.h} rows tall.`
        : `“${sectionLabel(section)}” moved to row ${p.y + 1}, column ${p.x + 1}.`
    )
    return true
  }

  return (
    <Stack gap="md">
      <Text id="dash-editor-keyboard-help" size="sm" c="dimmed">
        Drag a section by <IconArrowsMove size={14} style={{ verticalAlign: '-2px' }} /> to move it, and pull its corner
        or edges to resize it. With the keyboard, focus that handle: the arrow keys move the section and Shift with the
        arrow keys resizes it.
      </Text>
      {bookmarkOptions.length === 0 ? (
        <Text size="sm" c="orange">
          No bookmarks in your library yet. Add some in the library, then return here to pin them.
        </Text>
      ) : null}

      <BentoGrid
        layout={layout}
        editing
        onLayoutChange={apply}
        renderItem={(key) => {
          const idx = keys.indexOf(key)
          const section = draft.sections[idx]!
          return (
            <EditableSection
              section={section}
              bookmarkById={bookmarkById}
              folderName={folderName}
              onUpdateTitle={(title) => onUpdateTitle(idx, title)}
              onUpdateBookmarks={(ids) => onUpdateBookmarks(idx, ids)}
              onMovePin={(from, to) => onMovePin(idx, from, to)}
              onRemove={() => onRemoveSection(idx)}
              onOpenSettings={() => setSettingsKey(section.key)}
              onKeyboardStep={(code, resize) => keyboardMove(section, code, resize)}
            />
          )
        }}
      />
      <div role="status" aria-live="polite" className={editor.srOnly}>
        {announcement}
      </div>

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
