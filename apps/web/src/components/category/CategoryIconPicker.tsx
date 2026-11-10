import { useMemo, useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Group,
  Pagination,
  ScrollArea,
  Stack,
  Text,
  TextInput,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import { IconFolderFilled, IconSearch, IconX } from '@tabler/icons-react'
import { CATEGORY_ICON_OPTIONS, CategoryIcon, filterCategoryIcons, type CategoryIconKey } from '@/lib'
import styles from './CategoryIconPicker.module.css'

const PAGE_SIZE = 48

type Props = {
  value: CategoryIconKey | null
  onChange: (next: CategoryIconKey | null) => void
  label?: string
}

export function CategoryIconPicker({ value, onChange, label = 'Icon' }: Props): ReactElement {
  const [query, setQuery] = useState('')
  const [page, setPage] = useState(1)

  function search(next: string): void {
    setQuery(next)
    setPage(1)
  }

  const filtered = useMemo(() => filterCategoryIcons(query), [query])
  const selected = value ? CATEGORY_ICON_OPTIONS.find((o) => o.key === value) : undefined
  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE))
  const safePage = Math.min(page, totalPages)
  const pageItems = useMemo(() => {
    const start = (safePage - 1) * PAGE_SIZE
    return filtered.slice(start, start + PAGE_SIZE)
  }, [filtered, safePage])

  return (
    <Stack gap="xs">
      <Group justify="space-between" align="center">
        <Text size="sm" fw={500}>
          {label}
        </Text>
        <Group gap={6} aria-live="polite">
          <CategoryIcon icon={value} size={18} stroke={1.5} className={styles.current} aria-hidden />
          <Text size="xs" c="dimmed">
            {selected?.label ?? 'Default folder'}
          </Text>
          {value ? (
            <ActionIcon variant="subtle" color="gray" size="sm" aria-label="Clear icon" onClick={() => onChange(null)}>
              <IconX size={14} stroke={1.5} />
            </ActionIcon>
          ) : null}
        </Group>
      </Group>

      <TextInput
        type="search"
        autoComplete="off"
        data-1p-ignore
        data-lpignore="true"
        placeholder="Search icons"
        value={query}
        onChange={(e) => search(e.currentTarget.value)}
        leftSection={<IconSearch size={16} stroke={1.5} />}
        rightSection={
          query ? (
            <ActionIcon variant="subtle" color="gray" size="sm" aria-label="Clear search" onClick={() => search('')}>
              <IconX size={14} stroke={1.5} />
            </ActionIcon>
          ) : null
        }
        aria-label="Search icons"
      />

      <ScrollArea.Autosize mah={220} type="auto" offsetScrollbars>
        <div className={styles.grid} role="listbox" aria-label={label}>
          <Tooltip label="Default folder">
            <UnstyledButton
              type="button"
              className={`${styles.cell}${value == null ? ` ${styles.selected}` : ''}`}
              aria-selected={value == null}
              onClick={() => onChange(null)}
            >
              <IconFolderFilled size={20} stroke={1.5} aria-hidden />
            </UnstyledButton>
          </Tooltip>
          {pageItems.map(({ key, label: tip, Icon }) => (
            <Tooltip key={key} label={tip}>
              <UnstyledButton
                type="button"
                className={`${styles.cell}${value === key ? ` ${styles.selected}` : ''}`}
                aria-label={tip}
                aria-selected={value === key}
                onClick={() => onChange(key)}
              >
                <Icon size={20} stroke={1.5} aria-hidden />
              </UnstyledButton>
            </Tooltip>
          ))}
        </div>
      </ScrollArea.Autosize>

      {filtered.length === 0 ? (
        <Text size="xs" c="dimmed">
          No icons match “{query.trim()}”.
        </Text>
      ) : (
        <Group justify="space-between" align="center" gap="xs" wrap="nowrap">
          <Text size="xs" c="dimmed">
            {filtered.length} icon{filtered.length === 1 ? '' : 's'}
            {totalPages > 1 ? ` · page ${safePage}/${totalPages}` : ''}
          </Text>
          {totalPages > 1 ? (
            <Pagination size="sm" total={totalPages} value={safePage} onChange={setPage} aria-label="Icon pages" />
          ) : null}
        </Group>
      )}
    </Stack>
  )
}
