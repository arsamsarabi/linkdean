import { useMemo, useState, type KeyboardEvent, type ReactElement } from 'react'
import { Group, Kbd, Modal, Stack, Text, TextInput } from '@mantine/core'
import { IconBookmarkPlus, IconFolder, IconFolders, IconLockOpen, IconSearch, IconTags } from '@tabler/icons-react'
import { useNavigate, useRouterState, useSearch } from '@tanstack/react-router'
import { useCategories, useMe, useVaults } from '@/hooks'
import { buildLibrarySearch, categoryPath } from '@/lib'
import { buildPaletteItems, clampActiveIndex, FOLDER_ALL, FOLDER_UNCATEGORIZED, type PaletteItem } from '@/services'
import { useUi } from '@/stores'
import styles from './CommandPalette.module.css'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  opened: boolean
  onClose: () => void
}

function itemIcon(item: PaletteItem): ReactElement {
  switch (item.kind) {
    case 'add':
      return <IconBookmarkPlus {...iconSm} className={styles.itemIcon} />
    case 'unlock-vault':
      return <IconLockOpen {...iconSm} className={styles.itemIcon} />
    case 'search':
      return <IconSearch {...iconSm} className={styles.itemIcon} />
    case 'jump-all':
      return <IconFolders {...iconSm} className={styles.itemIcon} />
    case 'jump-uncategorized':
      return <IconTags {...iconSm} className={styles.itemIcon} />
    case 'jump-category':
      return <IconFolder {...iconSm} className={styles.itemIcon} />
  }
}

export function CommandPalette({ opened, onClose }: Props): ReactElement {
  const nav = useNavigate()
  const pathname = useRouterState({ select: (s) => s.location.pathname })
  const librarySearch = useSearch({ strict: false }) as { folder?: string; q?: string }
  const onLibrary = pathname.startsWith('/library')
  const showHidden = useUi((s) => s.showHidden)
  const cats = useCategories()
  const me = useMe()
  const vaults = useVaults(opened)
  const anyVaultLocked = (vaults.data?.vaults ?? []).some((v) => !v.unlocked)
  const showVaultInLibrary = me.data?.settings.show_vault_in_library ?? true
  const [query, setQuery] = useState('')
  const [active, setActive] = useState(0)
  const [openEpoch, setOpenEpoch] = useState(opened)
  if (opened !== openEpoch) {
    setOpenEpoch(opened)
    if (opened) {
      setQuery('')
      setActive(0)
    }
  }
  const [queryEpoch, setQueryEpoch] = useState(query)
  if (query !== queryEpoch) {
    setQueryEpoch(query)
    setActive(0)
  }

  const categoryRows = useMemo(() => {
    const list = cats.data || []
    const byId = new Map(list.map((c) => [c.id, c]))
    return list.map((c) => ({
      id: c.id,
      label: categoryPath(c, byId) + (c.is_vault ? ' (vault)' : '') + (c.is_hidden ? ' (hidden)' : ''),
      is_hidden: c.is_hidden,
      is_vault: c.is_vault,
    }))
  }, [cats.data])

  const items = useMemo(
    () => buildPaletteItems(query, categoryRows, { showHidden, anyVaultLocked }),
    [anyVaultLocked, query, categoryRows, showHidden]
  )

  function run(item: PaletteItem): void {
    onClose()
    switch (item.kind) {
      case 'add':
        void nav({
          to: '/library',
          search: buildLibrarySearch({
            create: true,
            folder: onLibrary ? librarySearch.folder : undefined,
          }),
        })
        return
      case 'unlock-vault':
        void nav({ to: '/settings', search: { tab: 'vault' } })
        return
      case 'search': {
        const q = query.trim()
        void nav({
          to: '/library',
          search: buildLibrarySearch({
            q: q || undefined,
            focus: 'search',
            folder: onLibrary ? librarySearch.folder : undefined,
          }),
        })
        return
      }
      case 'jump-all':
        void nav({
          to: '/library',
          search: buildLibrarySearch({ folder: FOLDER_ALL }),
        })
        return
      case 'jump-uncategorized':
        void nav({
          to: '/library',
          search: buildLibrarySearch({ folder: FOLDER_UNCATEGORIZED }),
        })
        return
      case 'jump-category':
        if (!item.categoryId) return
        void nav({
          to: item.isVault && !showVaultInLibrary ? '/vaults' : '/library',
          search: buildLibrarySearch({ folder: item.categoryId }),
        })
        return
    }
  }

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>): void {
    if (e.key === 'ArrowDown') {
      e.preventDefault()
      setActive((i) => clampActiveIndex(i + 1, items.length))
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      setActive((i) => clampActiveIndex(i - 1, items.length))
      return
    }
    if (e.key === 'Enter') {
      e.preventDefault()
      const item = items[active]
      if (item) run(item)
    }
  }

  return (
    <Modal
      opened={opened}
      onClose={onClose}
      centered
      size="lg"
      padding={0}
      withCloseButton={false}
      classNames={{ content: styles.modal }}
      overlayProps={{ backgroundOpacity: 0.4, blur: 4 }}
      transitionProps={{ transition: 'pop', duration: 150 }}
      aria-label="Command palette"
    >
      <TextInput
        className={styles.search}
        placeholder="Add, search, or jump to a folder…"
        value={query}
        onChange={(e) => setQuery(e.currentTarget.value)}
        onKeyDown={onKeyDown}
        leftSection={<IconSearch {...iconSm} />}
        variant="unstyled"
        size="md"
        px="md"
        py="sm"
        autoFocus
        data-autofocus
      />
      <div className={styles.list} role="listbox" aria-label="Commands">
        {items.length === 0 ? (
          <Text size="sm" c="dimmed" px="sm" py="md">
            No matches
          </Text>
        ) : (
          items.map((item, i) => (
            <button
              key={item.id}
              type="button"
              role="option"
              aria-selected={i === active}
              className={`${styles.item}${i === active ? ` ${styles.active}` : ''}`}
              onMouseEnter={() => setActive(i)}
              onClick={() => run(item)}
            >
              {itemIcon(item)}
              <Stack gap={2} style={{ minWidth: 0, flex: 1 }}>
                <Text size="sm" fw={500} truncate>
                  {item.label}
                </Text>
                {item.description ? (
                  <Text size="xs" c="dimmed" truncate>
                    {item.description}
                  </Text>
                ) : null}
              </Stack>
            </button>
          ))
        )}
      </div>
      <Group className={styles.footer} gap="md" justify="space-between">
        <Group gap="xs">
          <Kbd size="xs">↑↓</Kbd>
          <Text size="xs" c="dimmed">
            move
          </Text>
          <Kbd size="xs">↵</Kbd>
          <Text size="xs" c="dimmed">
            run
          </Text>
          <Kbd size="xs">esc</Kbd>
          <Text size="xs" c="dimmed">
            close
          </Text>
        </Group>
        <Group gap={4}>
          <Kbd size="xs">⌘K</Kbd>
          <Text size="xs" c="dimmed">
            toggle
          </Text>
        </Group>
      </Group>
    </Modal>
  )
}
