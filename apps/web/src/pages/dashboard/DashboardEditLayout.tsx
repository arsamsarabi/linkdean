import { type ReactElement } from 'react'
import {
  ActionIcon,
  Autocomplete,
  Badge,
  Button,
  Group,
  Menu,
  MultiSelect,
  NumberInput,
  Select,
  SimpleGrid,
  Stack,
  Switch,
  Text,
  TextInput,
  Tooltip,
} from '@mantine/core'
import {
  IconArrowDown,
  IconArrowUp,
  IconChevronDown,
  IconClockHour4,
  IconFlame,
  IconHistory,
  IconPinned,
  IconPlus,
  IconTrash,
} from '@tabler/icons-react'
import {
  DASHBOARD_SECTION_MAX_ITEMS,
  DASHBOARD_SECTION_MIN_ITEMS,
  type Bookmark,
  type DashboardOpenWindow,
  type DashboardSectionDisplay,
  type DashboardSectionKind,
} from '@/schemas'
import { sectionFillsItself, type DashboardDraft, type DashboardDraftSection } from '@/services'
import { DashboardSectionItems } from './DashboardSectionItems'

const iconSm = { size: 16, stroke: 1.5 } as const

const KIND_LABEL: Record<DashboardSectionKind, string> = {
  pinned: 'Pinned',
  latest: 'Latest',
  most_opened: 'Most opened',
  recently_visited: 'Recently visited',
}

const DISPLAY_OPTIONS: { value: DashboardSectionDisplay; label: string }[] = [
  { value: 'icons', label: 'Icons' },
  { value: 'small', label: 'Small' },
  { value: 'medium', label: 'Medium' },
  { value: 'large', label: 'Large' },
]

const WINDOW_OPTIONS: { value: DashboardOpenWindow; label: string }[] = [
  { value: 'last_30_days', label: 'Last 30 days' },
  { value: 'all_time', label: 'All time' },
]

const ADD_OPTIONS: { kind: DashboardSectionKind; hint: string; icon: ReactElement }[] = [
  { kind: 'pinned', hint: 'Bookmarks you pick', icon: <IconPinned {...iconSm} /> },
  { kind: 'latest', hint: 'Newest bookmarks that match', icon: <IconClockHour4 {...iconSm} /> },
  { kind: 'most_opened', hint: 'The ones you open most', icon: <IconFlame {...iconSm} /> },
  { kind: 'recently_visited', hint: 'The ones you opened last', icon: <IconHistory {...iconSm} /> },
]

export type SectionPatch = Partial<Omit<DashboardDraftSection, 'key' | 'kind' | 'bookmark_ids'>>

type Props = {
  draft: DashboardDraft
  bookmarkOptions: { value: string; label: string }[]
  bookmarkById: Map<string, Bookmark>
  /** The owner's folders outside vaults, for a section's source. */
  folderOptions: { value: string; label: string }[]
  tagOptions: string[]
  /** False on a shared dashboard or one someone else owns: only pinned sections can be added. */
  canAddSelfFilling: boolean
  viewerUserId: string | undefined
  onUpdateTitle: (idx: number, title: string) => void
  onUpdateBookmarks: (idx: number, bookmarkIds: string[]) => void
  onUpdateSection: (idx: number, patch: SectionPatch) => void
  onMoveSection: (idx: number, delta: -1 | 1) => void
  onRemoveSection: (idx: number) => void
  onAddSection: (kind: DashboardSectionKind) => void
}

function SourceFields({
  section,
  folderOptions,
  tagOptions,
  onChange,
}: {
  section: DashboardDraftSection
  folderOptions: Props['folderOptions']
  tagOptions: string[]
  onChange: (patch: SectionPatch) => void
}): ReactElement {
  const source = section.source
  const setSource = (patch: Partial<DashboardDraftSection['source']>): void =>
    onChange({ source: { ...source, ...patch } })
  return (
    <Stack gap="sm">
      <SimpleGrid cols={{ base: 1, sm: 3 }} spacing="sm">
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
  folderOptions: Props['folderOptions']
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

export function DashboardEditLayout({
  draft,
  bookmarkOptions,
  bookmarkById,
  folderOptions,
  tagOptions,
  canAddSelfFilling,
  viewerUserId,
  onUpdateTitle,
  onUpdateBookmarks,
  onUpdateSection,
  onMoveSection,
  onRemoveSection,
  onAddSection,
}: Props): ReactElement {
  return (
    <Stack gap="md">
      {bookmarkOptions.length === 0 ? (
        <Text size="sm" c="orange">
          No bookmarks in your library yet. Add some in the library, then return here to pin them.
        </Text>
      ) : null}
      {draft.sections.map((section, idx) => {
        const pinned = section.bookmark_ids.flatMap((id) => {
          const b = bookmarkById.get(id)
          return b ? [b] : []
        })
        return (
          <Stack
            key={section.key}
            gap="sm"
            p="md"
            style={{ border: '1px solid var(--mantine-color-default-border)', borderRadius: 8 }}
          >
            <Group justify="space-between" align="flex-end" wrap="wrap" gap="xs">
              <TextInput
                label={
                  <Group gap={6} component="span">
                    Section
                    <Badge size="xs" variant="light">
                      {KIND_LABEL[section.kind]}
                    </Badge>
                  </Group>
                }
                style={{ flex: '1 1 200px' }}
                value={section.title}
                onChange={(e) => onUpdateTitle(idx, e.currentTarget.value)}
              />
              <Select
                label="Show as"
                w={120}
                data={DISPLAY_OPTIONS}
                value={section.display}
                allowDeselect={false}
                onChange={(v) => v && onUpdateSection(idx, { display: v as DashboardSectionDisplay })}
              />
              <Group gap={4} wrap="nowrap">
                <Tooltip label="Move up">
                  <ActionIcon
                    variant="subtle"
                    aria-label="Move section up"
                    disabled={idx === 0}
                    onClick={() => onMoveSection(idx, -1)}
                  >
                    <IconArrowUp {...iconSm} />
                  </ActionIcon>
                </Tooltip>
                <Tooltip label="Move down">
                  <ActionIcon
                    variant="subtle"
                    aria-label="Move section down"
                    disabled={idx === draft.sections.length - 1}
                    onClick={() => onMoveSection(idx, 1)}
                  >
                    <IconArrowDown {...iconSm} />
                  </ActionIcon>
                </Tooltip>
                <Tooltip label="Remove section">
                  <ActionIcon
                    variant="subtle"
                    color="red"
                    aria-label="Remove section"
                    onClick={() => onRemoveSection(idx)}
                  >
                    <IconTrash {...iconSm} />
                  </ActionIcon>
                </Tooltip>
              </Group>
            </Group>

            {sectionFillsItself(section) ? (
              <SelfFillingFields
                section={section}
                folderOptions={folderOptions}
                tagOptions={tagOptions}
                onChange={(patch) => onUpdateSection(idx, patch)}
              />
            ) : (
              <>
                <MultiSelect
                  label="Bookmarks"
                  placeholder="Pick bookmarks from your library"
                  searchable
                  clearable
                  data={bookmarkOptions}
                  value={section.bookmark_ids}
                  onChange={(bookmark_ids) => onUpdateBookmarks(idx, bookmark_ids)}
                />
                {section.bookmark_ids.length === 0 ? (
                  <Text size="xs" c="dimmed">
                    No bookmarks yet — pick a few above.
                  </Text>
                ) : (
                  <DashboardSectionItems display={section.display} bookmarks={pinned} viewerUserId={viewerUserId} />
                )}
              </>
            )}
          </Stack>
        )
      })}
      {draft.sections.length === 0 ? (
        <Text size="sm" c="dimmed">
          No sections. Add one to start pinning links.
        </Text>
      ) : null}
      <Menu position="bottom-start" width={280}>
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
          {ADD_OPTIONS.map((o) => {
            const disabled = o.kind !== 'pinned' && !canAddSelfFilling
            return (
              <Menu.Item key={o.kind} leftSection={o.icon} disabled={disabled} onClick={() => onAddSection(o.kind)}>
                <Text size="sm">{KIND_LABEL[o.kind]}</Text>
                <Text size="xs" c="dimmed">
                  {disabled ? "Only on dashboards you own and haven't shared" : o.hint}
                </Text>
              </Menu.Item>
            )
          })}
        </Menu.Dropdown>
      </Menu>
    </Stack>
  )
}
