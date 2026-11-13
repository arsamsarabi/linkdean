import { useState, type ReactElement } from 'react'
import { ActionIcon, Popover, Select, Text, Tooltip } from '@mantine/core'
import { IconPlus } from '@tabler/icons-react'

type Props = {
  sectionTitle: string
  options: { value: string; label: string }[]
  /** Already in the section; left out of the list. */
  pinnedIds: string[]
  saving: boolean
  onPin: (bookmarkId: string) => Promise<boolean>
}

/** Pins one more bookmark to a section without opening the layout editor. */
export function QuickPinButton({ sectionTitle, options, pinnedIds, saving, onPin }: Props): ReactElement {
  const [opened, setOpened] = useState(false)
  const pinned = new Set(pinnedIds)
  const data = options.filter((o) => !pinned.has(o.value))

  return (
    <Popover opened={opened} onChange={setOpened} position="bottom-end" width={320} trapFocus withinPortal shadow="md">
      <Popover.Target>
        <Tooltip label="Pin a bookmark here" disabled={opened}>
          <ActionIcon
            variant="light"
            color="var(--dash-accent)"
            aria-label={`Pin a bookmark to ${sectionTitle}`}
            onClick={() => setOpened((o) => !o)}
          >
            <IconPlus size={16} stroke={1.8} />
          </ActionIcon>
        </Tooltip>
      </Popover.Target>
      <Popover.Dropdown>
        {data.length === 0 ? (
          <Text size="sm" c="dimmed">
            Every bookmark in your library is already here.
          </Text>
        ) : (
          <Select
            label={`Pin to ${sectionTitle}`}
            placeholder="Search your library"
            data={data}
            searchable
            value={null}
            disabled={saving}
            comboboxProps={{ withinPortal: false }}
            onChange={(id) => {
              if (!id) return
              void onPin(id).then((ok) => ok && setOpened(false))
            }}
            data-autofocus
          />
        )}
      </Popover.Dropdown>
    </Popover>
  )
}
