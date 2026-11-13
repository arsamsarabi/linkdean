import { type ReactElement } from 'react'
import {
  Autocomplete,
  Drawer,
  Group,
  MultiSelect,
  NumberInput,
  SegmentedControl,
  Select,
  SimpleGrid,
  Stack,
  Switch,
  Text,
  TextInput,
} from '@mantine/core'
import { useDrawerSize } from '@/hooks'
import {
  DASHBOARD_GRID_COLS,
  DASHBOARD_SECTION_MAX_H,
  DASHBOARD_SECTION_MAX_ITEMS,
  DASHBOARD_SECTION_MIN_H,
  DASHBOARD_SECTION_MIN_ITEMS,
  DASHBOARD_SECTION_MIN_W,
  type DashboardOpenWindow,
  type DashboardSectionDisplay,
} from '@/schemas'
import { sectionFillsItself, type DashboardDraftSection } from '@/services'
import { SECTION_KIND_LOOK } from './sectionKinds'
import { DISPLAY_OPTIONS } from './sectionOptions'

type Option = { value: string; label: string }

const WINDOW_OPTIONS: { value: DashboardOpenWindow; label: string }[] = [
  { value: 'last_30_days', label: 'Last 30 days' },
  { value: 'all_time', label: 'All time' },
]

export type SectionPatch = Partial<Omit<DashboardDraftSection, 'key' | 'kind' | 'bookmark_ids'>>

function SourceFields({
  section,
  folderOptions,
  tagOptions,
  onChange,
}: {
  section: DashboardDraftSection
  folderOptions: Option[]
  tagOptions: string[]
  onChange: (patch: SectionPatch) => void
}): ReactElement {
  const source = section.source
  const setSource = (patch: Partial<DashboardDraftSection['source']>): void =>
    onChange({ source: { ...source, ...patch } })
  return (
    <Stack gap="sm">
      <SimpleGrid cols={1} spacing="sm">
        <Select
          label="Folder"
          placeholder="Whole library"
          description="Includes its subfolders"
          data={folderOptions}
          value={source.category_id ?? null}
          onChange={(category_id) => setSource({ category_id })}
          searchable
          clearable
        />
        <Autocomplete
          label="Tag"
          placeholder="Any tag"
          description="Bookmarks with this tag"
          data={tagOptions}
          value={source.tag ?? ''}
          onChange={(tag) => setSource({ tag })}
        />
        <TextInput
          label="Website"
          placeholder="Any website"
          description="Such as github.com; subdomains match"
          value={source.site ?? ''}
          onChange={(e) => setSource({ site: e.currentTarget.value })}
        />
      </SimpleGrid>
      <Group gap="lg">
        <Switch
          label="Favorites only"
          checked={source.favorites}
          onChange={(e) => setSource({ favorites: e.currentTarget.checked })}
        />
        <Switch
          label="Unread only"
          checked={source.unread}
          onChange={(e) => setSource({ unread: e.currentTarget.checked })}
        />
      </Group>
    </Stack>
  )
}

function SelfFillingFields({
  section,
  folderOptions,
  tagOptions,
  onChange,
}: {
  section: DashboardDraftSection
  folderOptions: Option[]
  tagOptions: string[]
  onChange: (patch: SectionPatch) => void
}): ReactElement {
  return (
    <>
      {section.kind === 'recently_visited' ? null : (
        <SourceFields section={section} folderOptions={folderOptions} tagOptions={tagOptions} onChange={onChange} />
      )}
      <Group gap="sm" align="flex-end">
        <NumberInput
          label="Bookmarks to show"
          w={160}
          min={DASHBOARD_SECTION_MIN_ITEMS}
          max={DASHBOARD_SECTION_MAX_ITEMS}
          clampBehavior="strict"
          allowDecimal={false}
          value={section.item_limit}
          onChange={(v) => typeof v === 'number' && onChange({ item_limit: v })}
        />
        {section.kind === 'most_opened' ? (
          <Select
            label="Count opens over"
            w={160}
            data={WINDOW_OPTIONS}
            value={section.open_window}
            allowDeselect={false}
            onChange={(v) => v && onChange({ open_window: v as DashboardOpenWindow })}
          />
        ) : null}
      </Group>
      <Text size="xs" c="dimmed">
        {section.kind === 'recently_visited'
          ? 'Fills itself with the bookmarks you opened last. Vault bookmarks never show here.'
          : 'Fills itself from your library, minus vaults. A bookmark must match every filter you set.'}
      </Text>
    </>
  )
}

type DrawerProps = {
  /** The section being edited, or null when the drawer is closed. */
  section: DashboardDraftSection | null
  onClose: () => void
  bookmarkOptions: Option[]
  folderOptions: Option[]
  tagOptions: string[]
  onUpdateTitle: (title: string) => void
  onUpdateBookmarks: (bookmarkIds: string[]) => void
  onUpdateSection: (patch: SectionPatch) => void
}

/** One section's settings, beside the board while the layout is edited. */
export function SectionSettingsDrawer({
  section,
  onClose,
  bookmarkOptions,
  folderOptions,
  tagOptions,
  onUpdateTitle,
  onUpdateBookmarks,
  onUpdateSection,
}: DrawerProps): ReactElement {
  const drawerSize = useDrawerSize()
  return (
    <Drawer
      opened={section !== null}
      onClose={onClose}
      position="right"
      size={drawerSize}
      title={section ? `${SECTION_KIND_LOOK[section.kind].label} section` : 'Section'}
    >
      {section ? (
        <Stack gap="md">
          <TextInput
            label="Title"
            value={section.title}
            onChange={(e) => onUpdateTitle(e.currentTarget.value)}
            data-autofocus
          />
          <Stack gap={4}>
            <Text size="sm" fw={500}>
              Show as
            </Text>
            <SegmentedControl
              fullWidth
              data={DISPLAY_OPTIONS}
              value={section.display}
              onChange={(v) => onUpdateSection({ display: v as DashboardSectionDisplay })}
            />
          </Stack>
          <Stack gap={4}>
            <Group gap="sm" align="flex-end" grow>
              <NumberInput
                label="Width"
                description={`${DASHBOARD_SECTION_MIN_W} to ${DASHBOARD_GRID_COLS} columns`}
                min={DASHBOARD_SECTION_MIN_W}
                max={DASHBOARD_GRID_COLS}
                clampBehavior="strict"
                allowDecimal={false}
                value={section.layout.w}
                onChange={(v) =>
                  typeof v === 'number' &&
                  onUpdateSection({
                    layout: { ...section.layout, w: v, x: Math.min(section.layout.x, DASHBOARD_GRID_COLS - v) },
                  })
                }
              />
              <NumberInput
                label="Height"
                description={`${DASHBOARD_SECTION_MIN_H} to ${DASHBOARD_SECTION_MAX_H} rows`}
                min={DASHBOARD_SECTION_MIN_H}
                max={DASHBOARD_SECTION_MAX_H}
                clampBehavior="strict"
                allowDecimal={false}
                value={section.layout.h}
                onChange={(v) => typeof v === 'number' && onUpdateSection({ layout: { ...section.layout, h: v } })}
              />
            </Group>
            <Text size="xs" c="dimmed">
              Or drag the section&apos;s corner on the board. Phones stack sections at their natural height.
            </Text>
          </Stack>
          {sectionFillsItself(section) ? (
            <SelfFillingFields
              section={section}
              folderOptions={folderOptions}
              tagOptions={tagOptions}
              onChange={onUpdateSection}
            />
          ) : (
            <MultiSelect
              label="Bookmarks"
              description="Drag them on the board to change the order."
              placeholder="Pick bookmarks from your library"
              searchable
              clearable
              data={bookmarkOptions}
              value={section.bookmark_ids}
              onChange={onUpdateBookmarks}
            />
          )}
        </Stack>
      ) : null}
    </Drawer>
  )
}
