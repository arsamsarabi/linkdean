import { useMemo, useState, type ReactElement } from 'react'
import { Box, Button, Group, SimpleGrid, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import { useMediaQuery } from '@mantine/hooks'
import { IconFolderPlus, IconFolders } from '@tabler/icons-react'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { CategoryDrawer, FolderTile, QueryErrorAlert } from '@/components'
import { useCategories, useCategoryFolderActions } from '@/hooks'
import { buildCategoryTree, canWriteCategory } from '@/lib'
import type { Category } from '@/schemas'
import classes from './CategoriesPage.module.css'
import { CategoryDetail } from './CategoryDetail'
import { CategoryTree } from './CategoryTree'

function childCountMap(cats: Category[]): Map<string, number> {
  const map = new Map<string, number>()
  for (const c of cats) {
    if (!c.parent_id) continue
    map.set(c.parent_id, (map.get(c.parent_id) || 0) + 1)
  }
  return map
}

function ancestorsOf(cat: Category, byId: Map<string, Category>): Category[] {
  const out: Category[] = []
  let parent = cat.parent_id ? byId.get(cat.parent_id) : undefined
  while (parent && out.length < 64) {
    out.unshift(parent)
    parent = parent.parent_id ? byId.get(parent.parent_id) : undefined
  }
  return out
}

function subcategoryLabel(n: number): string {
  return n === 0 ? 'No subcategories' : `${n} subcategor${n === 1 ? 'y' : 'ies'}`
}

/** Category management: the whole tree on one side, the chosen category and its actions on the other. */
export function CategoriesPage(): ReactElement {
  const nav = useNavigate()
  const search = useSearch({ strict: false }) as { id?: string }
  const wide = useMediaQuery('(min-width: 62em)', true)
  const cats = useCategories(true)
  // `undefined` keeps the drawer shut; `null` opens it at the root.
  const [createParent, setCreateParent] = useState<string | null | undefined>(undefined)

  const all = useMemo(() => cats.data ?? [], [cats.data])
  const byId = useMemo(() => new Map(all.map((c) => [c.id, c])), [all])
  const counts = useMemo(() => childCountMap(all), [all])
  const tree = useMemo(() => buildCategoryTree(all), [all])
  const owned = useMemo(() => tree.filter((n) => n.cat.access === 'owner'), [tree])
  const shared = useMemo(() => tree.filter((n) => n.cat.access !== 'owner'), [tree])

  const selected = search.id ? (byId.get(search.id) ?? null) : null

  function select(id: string | null): void {
    void nav({ to: '/categories', search: id ? { id } : {} })
  }

  const actions = useCategoryFolderActions({
    onDeleted: (id) => {
      if (id === search.id) select(byId.get(id)?.parent_id ?? null)
    },
  })

  const ancestors = useMemo(() => (selected ? ancestorsOf(selected, byId) : []), [selected, byId])
  const subcategories = useMemo(
    () => (selected ? all.filter((c) => c.parent_id === selected.id).sort((a, b) => a.name.localeCompare(b.name)) : []),
    [all, selected]
  )

  const showTree = wide || !selected
  const showDetail = wide || !!selected

  const overview = (
    <Stack gap="sm">
      <Text size="sm" c="dimmed">
        Pick a category to see its details and everything you can do with it.
      </Text>
      {tree.length > 0 ? (
        <SimpleGrid cols={{ base: 2, sm: 3, lg: 4 }} spacing="md">
          {tree.map(({ cat }) => (
            <FolderTile
              key={cat.id}
              id={cat.id}
              name={cat.name}
              countLabel={subcategoryLabel(counts.get(cat.id) ?? 0)}
              onOpen={select}
              isVault={cat.is_vault}
              isHidden={cat.is_hidden}
              icon={cat.icon}
              access={cat.access}
              sharedOut={cat.shared_out}
              sharedWith={cat.shared_with}
              ownerDisplayName={cat.owner_display_name}
              deletedAt={cat.deleted_at}
            />
          ))}
        </SimpleGrid>
      ) : null}
    </Stack>
  )

  return (
    <Stack gap="xl">
      <Group justify="space-between" wrap="wrap" gap="sm">
        <Group gap="md">
          <ThemeIcon size={40} radius="md" variant="light" color="violet">
            <IconFolders size={22} stroke={1.5} />
          </ThemeIcon>
          <div>
            <Title order={2}>Categories</Title>
            <Text size="sm" c="dimmed">
              Nest, edit and share the categories your bookmarks live in.
            </Text>
          </div>
        </Group>
        <Button leftSection={<IconFolderPlus size={16} stroke={1.5} />} onClick={() => setCreateParent(null)}>
          New category
        </Button>
      </Group>

      {cats.isError ? <QueryErrorAlert error={cats.error} onRetry={() => void cats.refetch()} /> : null}

      <Box className={classes.layout}>
        {showTree ? (
          <CategoryTree
            owned={owned}
            shared={shared}
            all={all}
            selectedId={selected?.id}
            loading={cats.isLoading}
            onSelect={select}
          />
        ) : null}
        {showDetail ? (
          <Box miw={0}>
            {selected ? (
              <CategoryDetail
                category={selected}
                ancestors={ancestors}
                subcategories={subcategories}
                childCounts={counts}
                onSelect={select}
                onBack={wide ? undefined : () => select(null)}
                actions={{
                  onEdit: () => actions.openEdit(selected),
                  onNewSubcategory: () => setCreateParent(selected.id),
                  onShare: () => actions.openShare(selected),
                  onDuplicate: () => actions.forkCategory(selected),
                  onMoveToVault: () => actions.openMove(selected),
                  onSocial: () => actions.openSocial(selected),
                  onDelete: () => actions.openDelete(selected.id, selected.is_vault),
                  duplicatePending: actions.forkPending,
                }}
              />
            ) : (
              overview
            )}
          </Box>
        ) : null}
      </Box>

      <CategoryDrawer
        opened={createParent !== undefined}
        onClose={() => setCreateParent(undefined)}
        categories={all}
        defaultParentId={
          createParent && byId.get(createParent) && canWriteCategory(byId.get(createParent)!) ? createParent : null
        }
      />

      {actions.modals}
    </Stack>
  )
}
