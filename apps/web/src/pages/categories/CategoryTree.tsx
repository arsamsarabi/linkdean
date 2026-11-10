import { useState, type ReactElement } from 'react'
import { CloseButton, NavLink, Paper, ScrollArea, Skeleton, Stack, Text, TextInput } from '@mantine/core'
import { IconSearch } from '@tabler/icons-react'
import { CategoryIcon, filterFolders, type CatNode } from '@/lib'
import type { Category } from '@/schemas'

type Props = {
  owned: CatNode[]
  shared: CatNode[]
  all: Category[]
  selectedId: string | undefined
  loading: boolean
  onSelect: (id: string) => void
}

function containsCategory(node: CatNode, id: string | undefined): boolean {
  return !!id && (node.cat.id === id || node.children.some((c) => containsCategory(c, id)))
}

function TreeItem({
  node,
  selectedId,
  onSelect,
}: { node: CatNode } & Pick<Props, 'selectedId' | 'onSelect'>): ReactElement {
  return (
    <NavLink
      component="button"
      label={node.cat.name}
      leftSection={<CategoryIcon icon={node.cat.icon} size={18} stroke={1.5} aria-hidden />}
      active={selectedId === node.cat.id}
      defaultOpened={node.children.length > 0 && containsCategory(node, selectedId) && node.cat.id !== selectedId}
      childrenOffset={14}
      onClick={() => onSelect(node.cat.id)}
    >
      {node.children.length > 0
        ? node.children.map((c) => <TreeItem key={c.cat.id} node={c} selectedId={selectedId} onSelect={onSelect} />)
        : undefined}
    </NavLink>
  )
}

function Section({
  label,
  nodes,
  selectedId,
  onSelect,
}: { label: string; nodes: CatNode[] } & Pick<Props, 'selectedId' | 'onSelect'>): ReactElement | null {
  if (nodes.length === 0) return null
  return (
    <Stack gap={2}>
      <Text size="xs" fw={600} c="dimmed" tt="uppercase" px="sm" pt="xs" pb={4}>
        {label}
      </Text>
      {nodes.map((n) => (
        <TreeItem key={n.cat.id} node={n} selectedId={selectedId} onSelect={onSelect} />
      ))}
    </Stack>
  )
}

/** Every category as an expandable tree, with a name filter that flattens matches to their paths. */
export function CategoryTree({ owned, shared, all, selectedId, loading, onSelect }: Props): ReactElement {
  const [query, setQuery] = useState('')
  const matches = filterFolders(all, query)
  const filtering = query.trim() !== ''

  let body: ReactElement
  if (loading) {
    body = (
      <Stack gap={6} p="sm">
        <Skeleton height={24} />
        <Skeleton height={24} />
        <Skeleton height={24} width="70%" />
      </Stack>
    )
  } else if (filtering) {
    body =
      matches.length > 0 ? (
        <Stack gap={2}>
          {matches.map(({ cat, path }) => (
            <NavLink
              key={cat.id}
              component="button"
              label={cat.name}
              description={path !== cat.name ? path : undefined}
              leftSection={<CategoryIcon icon={cat.icon} size={18} stroke={1.5} aria-hidden />}
              active={selectedId === cat.id}
              onClick={() => onSelect(cat.id)}
            />
          ))}
        </Stack>
      ) : (
        <Text size="sm" c="dimmed" p="sm">
          No categories match.
        </Text>
      )
  } else if (owned.length === 0 && shared.length === 0) {
    body = (
      <Text size="sm" c="dimmed" p="sm">
        No categories yet.
      </Text>
    )
  } else {
    body = (
      <Stack gap="xs">
        <Section
          label={shared.length > 0 ? 'Your categories' : 'Categories'}
          nodes={owned}
          selectedId={selectedId}
          onSelect={onSelect}
        />
        <Section label="Shared with you" nodes={shared} selectedId={selectedId} onSelect={onSelect} />
      </Stack>
    )
  }

  return (
    <Paper withBorder p="xs">
      <Stack gap="xs">
        <TextInput
          type="search"
          name="category-filter"
          autoComplete="off"
          placeholder="Filter categories"
          aria-label="Filter categories"
          value={query}
          onChange={(e) => setQuery(e.currentTarget.value)}
          leftSection={<IconSearch size={14} stroke={1.5} />}
          rightSection={query ? <CloseButton size="sm" aria-label="Clear filter" onClick={() => setQuery('')} /> : null}
        />
        <ScrollArea.Autosize mah="calc(100dvh - 260px)" type="auto" offsetScrollbars>
          <nav aria-label="Categories">{body}</nav>
        </ScrollArea.Autosize>
      </Stack>
    </Paper>
  )
}
