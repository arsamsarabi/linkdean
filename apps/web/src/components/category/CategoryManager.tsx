import { useMemo, useState, type ReactElement } from 'react'
import { Button, Group, SimpleGrid, Skeleton, Stack, Text } from '@mantine/core'
import { IconFolderPlus } from '@tabler/icons-react'
import { ExplorerBreadcrumb, FolderTile } from '@/components/library-explorer'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { useCategories, useCategoryFolderActions } from '@/hooks'
import { buildCategoryTree } from '@/lib'
import type { Category } from '@/schemas'
import { FOLDER_ALL, folderBreadcrumb } from '@/services'
import { CategoryDrawer } from './CategoryDrawer'

const iconSm = { size: 16, stroke: 1.5 } as const

function childCountMap(cats: Category[]): Map<string, number> {
  const map = new Map<string, number>()
  for (const c of cats) {
    if (!c.parent_id) continue
    map.set(c.parent_id, (map.get(c.parent_id) || 0) + 1)
  }
  return map
}

function folderCountLabel(n: number): string {
  return n === 0 ? 'Empty' : `${n} folder${n === 1 ? '' : 's'}`
}

export function CategoryManager(): ReactElement {
  const [folderId, setFolderId] = useState<string>(FOLDER_ALL)
  const [createOpen, setCreateOpen] = useState(false)

  const cats = useCategories(true)
  const actions = useCategoryFolderActions({
    onDeleted: (id) => {
      if (folderId === id) setFolderId(FOLDER_ALL)
    },
  })

  const all = cats.data || []
  const byId = useMemo(() => new Map(all.map((c) => [c.id, c])), [all])
  const counts = useMemo(() => childCountMap(all), [all])
  const tree = useMemo(() => buildCategoryTree(all), [all])
  const ownedRoots = useMemo(() => tree.filter((n) => n.cat.access === 'owner'), [tree])
  const sharedRoots = useMemo(() => tree.filter((n) => n.cat.access !== 'owner'), [tree])

  const crumbs = useMemo(() => {
    const segs = folderBreadcrumb(folderId === FOLDER_ALL ? undefined : folderId, byId)
    if (segs[0]?.id === FOLDER_ALL) segs[0] = { id: FOLDER_ALL, name: 'Categories' }
    return segs
  }, [folderId, byId])

  const atRoot = folderId === FOLDER_ALL
  const current = atRoot ? null : byId.get(folderId) || null

  const childCats = useMemo(() => {
    if (atRoot) return []
    return all.filter((c) => c.parent_id === folderId).sort((a, b) => a.name.localeCompare(b.name))
  }, [all, atRoot, folderId])

  const defaultParentId = useMemo(() => {
    if (atRoot) return null
    if (!current) return null
    if (current.access === 'owner' || current.access === 'write') return current.id
    return null
  }, [atRoot, current])

  function tileMenu(cat: Category) {
    return {
      onRename: () => actions.openRename(cat),
      onShare: () => actions.openShare(cat),
      onFork: () => actions.forkCategory(cat),
      onDelete: () => actions.openDelete(cat.id),
      forkPending: actions.forkPending,
    }
  }

  function renderGrid(list: Category[]): ReactElement {
    if (list.length === 0) {
      return <Text c="dimmed">No folders here.</Text>
    }
    return (
      <SimpleGrid cols={{ base: 2, sm: 3, md: 4 }} spacing="md">
        {list.map((cat) => (
          <FolderTile
            key={cat.id}
            id={cat.id}
            name={cat.name}
            countLabel={folderCountLabel(counts.get(cat.id) || 0)}
            onOpen={setFolderId}
            isVault={cat.is_vault}
            isHidden={cat.is_hidden}
            access={cat.access}
            sharedOut={cat.shared_out}
            sharedWith={cat.shared_with}
            ownerDisplayName={cat.owner_display_name}
            deletedAt={cat.deleted_at}
            menu={tileMenu(cat)}
          />
        ))}
      </SimpleGrid>
    )
  }

  return (
    <Stack gap="lg">
      <Text size="sm" c="dimmed">
        Nested folders for the library. Browse like the explorer; add via the drawer. Share owned
        folders with other users on this instance.
      </Text>

      {cats.isError ? <QueryErrorAlert error={cats.error} onRetry={() => void cats.refetch()} /> : null}

      <Group justify="space-between" align="center" wrap="wrap" gap="sm">
        <ExplorerBreadcrumb segments={crumbs} onNavigate={setFolderId} />
        <Button leftSection={<IconFolderPlus {...iconSm} />} onClick={() => setCreateOpen(true)}>
          New category
        </Button>
      </Group>

      {cats.isLoading ? (
        <SimpleGrid cols={{ base: 2, sm: 3, md: 4 }} spacing="md">
          <Skeleton height={120} />
          <Skeleton height={120} />
          <Skeleton height={120} width="70%" />
        </SimpleGrid>
      ) : atRoot ? (
        <Stack gap="md">
          {ownedRoots.length === 0 && sharedRoots.length === 0 ? (
            <Text c="dimmed">No categories yet.</Text>
          ) : null}
          {ownedRoots.length > 0 ? (
            <Stack gap="sm">
              {sharedRoots.length > 0 ? (
                <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                  Your folders
                </Text>
              ) : null}
              {renderGrid(ownedRoots.map((n) => n.cat))}
            </Stack>
          ) : null}
          {sharedRoots.length > 0 ? (
            <Stack gap="sm">
              <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                Shared with you
              </Text>
              {renderGrid(sharedRoots.map((n) => n.cat))}
            </Stack>
          ) : null}
        </Stack>
      ) : (
        renderGrid(childCats)
      )}

      <CategoryDrawer
        opened={createOpen}
        onClose={() => setCreateOpen(false)}
        categories={all}
        defaultParentId={defaultParentId}
      />

      {actions.modals}
    </Stack>
  )
}
