import { useCallback, useMemo, useState, type KeyboardEvent, type ReactElement } from 'react'
import { ActionIcon, Badge, Text, Tooltip } from '@mantine/core'
import { IconChevronDown, IconChevronRight, IconLock } from '@tabler/icons-react'
import { FOLDER_ALL, type ExplorerFolderId, type ExplorerTreeNode } from '@/services'
import styles from './ExplorerFolderTree.module.css'

type Props = {
  tree: ExplorerTreeNode[]
  selectedId: ExplorerFolderId
  onSelect: (id: ExplorerFolderId) => void
}

function treeSignature(tree: ExplorerTreeNode[]): string {
  const walk = (nodes: ExplorerTreeNode[]): string =>
    nodes.map((n) => `${n.id}:${n.children.length > 0 ? walk(n.children) : ''}`).join('|')
  return walk(tree)
}

function defaultExpanded(tree: ExplorerTreeNode[]): Set<string> {
  // Expand All + first level only (collapse deep by default).
  const set = new Set<string>([FOLDER_ALL])
  const roots = tree[0]?.children || []
  for (const n of roots) {
    if (n.children.length > 0) set.add(n.id)
  }
  return set
}

function flattenVisible(nodes: ExplorerTreeNode[], expanded: Set<string>, depth = 1): { node: ExplorerTreeNode; depth: number }[] {
  const rows: { node: ExplorerTreeNode; depth: number }[] = []
  for (const n of nodes) {
    rows.push({ node: n, depth })
    if (n.children.length > 0 && expanded.has(n.id)) {
      rows.push(...flattenVisible(n.children, expanded, depth + 1))
    }
  }
  return rows
}

export function ExplorerFolderTree({ tree, selectedId, onSelect }: Props): ReactElement {
  const signature = treeSignature(tree)
  const [expanded, setExpanded] = useState<Set<string>>(() => defaultExpanded(tree))
  const [expandedFor, setExpandedFor] = useState(signature)

  if (expandedFor !== signature) {
    setExpandedFor(signature)
    setExpanded(defaultExpanded(tree))
  }

  const visible = useMemo(() => flattenVisible(tree, expanded), [tree, expanded])

  const toggle = useCallback((id: string) => {
    setExpanded((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }, [])

  function onKeyDown(e: KeyboardEvent<HTMLUListElement>): void {
    const idx = visible.findIndex((r) => r.node.id === selectedId)
    if (idx < 0 && visible.length === 0) return
    const current = visible[idx]?.node

    if (e.key === 'ArrowDown') {
      e.preventDefault()
      const next = visible[Math.min(idx + 1, visible.length - 1)]
      if (next) onSelect(next.node.id)
      return
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault()
      const prev = visible[Math.max(idx - 1, 0)]
      if (prev) onSelect(prev.node.id)
      return
    }
    if (e.key === 'ArrowRight' && current) {
      e.preventDefault()
      if (current.children.length > 0 && !expanded.has(current.id)) toggle(current.id)
      else if (current.children[0]) onSelect(current.children[0].id)
      return
    }
    if (e.key === 'ArrowLeft' && current) {
      e.preventDefault()
      if (current.children.length > 0 && expanded.has(current.id)) toggle(current.id)
      else {
        const parent = findParent(tree, current.id)
        if (parent) onSelect(parent.id)
      }
      return
    }
    if (e.key === 'Home') {
      e.preventDefault()
      if (visible[0]) onSelect(visible[0].node.id)
      return
    }
    if (e.key === 'End') {
      e.preventDefault()
      const last = visible[visible.length - 1]
      if (last) onSelect(last.node.id)
    }
  }

  return (
    <ul role="tree" aria-label="Folders" className={styles.tree} onKeyDown={onKeyDown}>
      {tree.map((n) => (
        <TreeItem
          key={n.id}
          node={n}
          depth={1}
          selectedId={selectedId}
          expanded={expanded}
          onToggle={toggle}
          onSelect={onSelect}
        />
      ))}
    </ul>
  )
}

function findParent(nodes: ExplorerTreeNode[], id: string, parent: ExplorerTreeNode | null = null): ExplorerTreeNode | null {
  for (const n of nodes) {
    if (n.id === id) return parent
    const found = findParent(n.children, id, n)
    if (found) return found
  }
  return null
}

type ItemProps = {
  node: ExplorerTreeNode
  depth: number
  selectedId: ExplorerFolderId
  expanded: Set<string>
  onToggle: (id: string) => void
  onSelect: (id: ExplorerFolderId) => void
}

function TreeItem({ node, depth, selectedId, expanded, onToggle, onSelect }: ItemProps): ReactElement {
  const hasKids = node.children.length > 0
  const isExpanded = expanded.has(node.id)
  const selected = selectedId === node.id

  return (
    <li
      role="treeitem"
      aria-selected={selected}
      aria-expanded={hasKids ? isExpanded : undefined}
      aria-level={depth}
      className={styles.item}
    >
      <div className={`${styles.row} ${selected ? styles.rowSelected : ''}`} style={{ paddingLeft: (depth - 1) * 12 }}>
        {hasKids ? (
          <ActionIcon
            className={styles.chevron}
            size="sm"
            variant="subtle"
            color="gray"
            aria-label={isExpanded ? `Collapse ${node.name}` : `Expand ${node.name}`}
            onClick={(e) => {
              e.stopPropagation()
              onToggle(node.id)
            }}
          >
            {isExpanded ? <IconChevronDown size={14} stroke={1.5} /> : <IconChevronRight size={14} stroke={1.5} />}
          </ActionIcon>
        ) : (
          <span className={styles.chevron} style={{ width: 28 }} aria-hidden />
        )}
        <button type="button" className={styles.label} onClick={() => onSelect(node.id)}>
          <Text component="span" size="sm" fw={selected ? 600 : 400} lineClamp={1}>
            {node.name}
          </Text>
          {node.isVault ? (
            <Tooltip label="Vault">
              <Badge ml={6} size="xs" variant="light" color="grape" leftSection={<IconLock size={10} stroke={1.5} />}>
                V
              </Badge>
            </Tooltip>
          ) : null}
        </button>
        <span className={styles.count} aria-hidden>
          {node.bookmarkCount}
        </span>
      </div>
      {hasKids && isExpanded ? (
        <ul role="group" className={styles.group}>
          {node.children.map((c) => (
            <TreeItem
              key={c.id}
              node={c}
              depth={depth + 1}
              selectedId={selectedId}
              expanded={expanded}
              onToggle={onToggle}
              onSelect={onSelect}
            />
          ))}
        </ul>
      ) : null}
    </li>
  )
}
