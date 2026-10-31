import { useMemo, useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Badge,
  Button,
  Checkbox,
  Group,
  ScrollArea,
  Select,
  Skeleton,
  Stack,
  Text,
  TextInput,
  Tooltip,
  UnstyledButton,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCopy, IconFolder, IconFolderPlus, IconShare, IconTrash } from '@tabler/icons-react'
import { ConfirmModal } from '@/components/confirm-modal'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { ShareBadge } from '@/components/share-badge'
import {
  useCategories,
  useCreateCategory,
  useDeleteCategory,
  useForkCategory,
  useUpdateCategory,
} from '@/hooks'
import { buildCategoryTree, categorySelectData, type CatNode } from '@/lib'
import type { Category } from '@/schemas'
import { ShareCategoryDrawer } from './ShareCategoryDrawer'

const iconSm = { size: 16, stroke: 1.5 } as const

export function CategoryManager(): ReactElement {
  const [name, setName] = useState('')
  const [parentId, setParentId] = useState<string | null>(null)
  const [hidden, setHidden] = useState(false)
  const [vault, setVault] = useState(false)
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [shareCat, setShareCat] = useState<Category | null>(null)

  const cats = useCategories(true)
  const create = useCreateCategory()
  const update = useUpdateCategory()
  const remove = useDeleteCategory()
  const fork = useForkCategory()

  const tree = useMemo(() => buildCategoryTree(cats.data || []), [cats.data])
  const ownedRoots = useMemo(() => tree.filter((n) => n.cat.access === 'owner'), [tree])
  const sharedRoots = useMemo(() => tree.filter((n) => n.cat.access !== 'owner'), [tree])
  const splitShared = sharedRoots.length > 0
  const parentOptions = useMemo(
    () => [
      { value: '', label: '- root -' },
      ...categorySelectData((cats.data || []).filter((c) => c.access === 'owner' || c.access === 'write')),
    ],
    [cats.data]
  )
  const parentCat = useMemo(
    () => (parentId ? (cats.data || []).find((c) => c.id === parentId) : null),
    [cats.data, parentId]
  )
  const canCreateVault = !parentCat || parentCat.access === 'owner'

  async function renameCategory(id: string, next: string): Promise<void> {
    try {
      await update.mutateAsync({ id, body: { name: next } })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onCreate(): Promise<void> {
    try {
      await create.mutateAsync({
        name: name.trim(),
        parent_id: parentId || null,
        is_hidden: hidden,
        is_vault: vault,
      })
      setName('')
      setParentId(null)
      setHidden(false)
      setVault(false)
      notifications.show({ message: 'Category created' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function confirmDelete(): Promise<void> {
    if (!deleteId) return
    try {
      await remove.mutateAsync(deleteId)
      notifications.show({ message: 'Category deleted' })
      setDeleteId(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onFork(cat: Category): Promise<void> {
    try {
      const res = await fork.mutateAsync({ id: cat.id })
      notifications.show({
        message: `Copied “${res.category.name}” (${res.bookmarks_copied} bookmark${res.bookmarks_copied === 1 ? '' : 's'})`,
      })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Stack gap="lg">
      <Text size="sm" c="dimmed">
        Nested folders for the library. Manage here; library filters by category. Share owned
        folders with other users on this instance.
      </Text>

      {cats.isError ? <QueryErrorAlert error={cats.error} onRetry={() => void cats.refetch()} /> : null}

      {cats.isLoading ? (
        <Stack gap="xs">
          <Skeleton height={28} />
          <Skeleton height={28} width="80%" />
          <Skeleton height={28} width="60%" />
        </Stack>
      ) : (
        <ScrollArea.Autosize mah={320} type="auto" offsetScrollbars>
          <Stack gap="md">
            {tree.length === 0 ? <Text c="dimmed">No categories yet.</Text> : null}
            {ownedRoots.length > 0 ? (
              <Stack gap={6}>
                {splitShared ? (
                  <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                    Your folders
                  </Text>
                ) : null}
                {ownedRoots.map((n) => (
                  <CatRow
                    key={n.cat.id}
                    node={n}
                    depth={0}
                    onDelete={setDeleteId}
                    onShare={setShareCat}
                    onRename={renameCategory}
                    onFork={onFork}
                    forkPending={fork.isPending}
                  />
                ))}
              </Stack>
            ) : null}
            {sharedRoots.length > 0 ? (
              <Stack gap={6}>
                <Text size="xs" c="dimmed" tt="uppercase" fw={600}>
                  Shared with you
                </Text>
                {sharedRoots.map((n) => (
                  <CatRow
                    key={n.cat.id}
                    node={n}
                    depth={0}
                    onDelete={setDeleteId}
                    onShare={setShareCat}
                    onRename={renameCategory}
                    onFork={onFork}
                    forkPending={fork.isPending}
                  />
                ))}
              </Stack>
            ) : null}
          </Stack>
        </ScrollArea.Autosize>
      )}

      <Stack gap="md">
        <TextInput
          label="New category"
          placeholder="Name"
          value={name}
          onChange={(e) => setName(e.currentTarget.value)}
        />
        <Select
          label="Parent"
          data={parentOptions}
          value={parentId ?? ''}
          onChange={(v) => {
            const next = v || null
            setParentId(next)
            const p = next ? (cats.data || []).find((c) => c.id === next) : null
            if (p && p.access !== 'owner') setVault(false)
          }}
          searchable
          clearable
        />
        <Group gap="lg">
          <Checkbox label="Hidden" checked={hidden} onChange={(e) => setHidden(e.currentTarget.checked)} />
          <Checkbox
            label="Vault"
            checked={vault}
            disabled={!canCreateVault}
            onChange={(e) => setVault(e.currentTarget.checked)}
          />
        </Group>
        <Button
          leftSection={<IconFolderPlus {...iconSm} />}
          onClick={() => void onCreate()}
          disabled={!name.trim()}
          loading={create.isPending}
        >
          Add category
        </Button>
      </Stack>

      <ConfirmModal
        opened={!!deleteId}
        onClose={() => setDeleteId(null)}
        onConfirm={() => void confirmDelete()}
        title="Delete category?"
        body="Bookmarks in this category keep their links but lose this folder assignment."
        confirmLabel="Delete"
        loading={remove.isPending}
      />

      <ShareCategoryDrawer
        opened={!!shareCat}
        onClose={() => setShareCat(null)}
        categoryId={shareCat?.id ?? null}
        categoryName={shareCat?.name ?? ''}
      />
    </Stack>
  )
}

type CatRowProps = {
  node: CatNode
  depth: number
  onDelete: (id: string) => void
  onShare: (cat: Category) => void
  onRename: (id: string, name: string) => Promise<void>
  onFork: (cat: Category) => void
  forkPending?: boolean
}

function CatRow({
  node,
  depth,
  onDelete,
  onShare,
  onRename,
  onFork,
  forkPending,
}: CatRowProps): ReactElement {
  const [editing, setEditing] = useState(false)
  const [name, setName] = useState(node.cat.name)
  const canWrite = node.cat.access === 'owner' || node.cat.access === 'write'
  const isOwner = node.cat.access === 'owner'

  async function commitRename(): Promise<void> {
    setEditing(false)
    if (name.trim() && name.trim() !== node.cat.name) {
      await onRename(node.cat.id, name.trim())
    } else {
      setName(node.cat.name)
    }
  }

  return (
    <>
      <Group gap="xs" wrap="nowrap" style={{ paddingLeft: depth * 16 }}>
        <IconFolder size={14} stroke={1.5} style={{ flexShrink: 0 }} />
        {editing && canWrite ? (
          <TextInput
            size="xs"
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            onBlur={() => void commitRename()}
            onKeyDown={(e) => {
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
              if (e.key === 'Escape') {
                setName(node.cat.name)
                setEditing(false)
              }
            }}
            style={{ flex: 1 }}
            autoFocus
          />
        ) : canWrite ? (
          <UnstyledButton
            type="button"
            onClick={() => setEditing(true)}
            aria-label={`Rename ${node.cat.name}`}
            style={{ flex: 1, textAlign: 'left' }}
          >
            <Text size="sm">{node.cat.name}</Text>
          </UnstyledButton>
        ) : (
          <Text size="sm" style={{ flex: 1 }}>
            {node.cat.name}
          </Text>
        )}
        {node.cat.access === 'read' || node.cat.access === 'write' ? (
          <ShareBadge access={node.cat.access} />
        ) : null}
        {node.cat.access === 'owner' && node.cat.shared_out ? <ShareBadge outgoing /> : null}
        {node.cat.is_hidden ? (
          <Badge size="xs" variant="outline" color="gray">
            hidden
          </Badge>
        ) : null}
        {node.cat.is_vault ? (
          <Badge size="xs" variant="light">
            vault
          </Badge>
        ) : null}
        {!node.cat.is_vault ? (
          <Tooltip label="Duplicate list into your library">
            <ActionIcon
              size="sm"
              variant="subtle"
              aria-label="Duplicate list"
              loading={forkPending}
              onClick={() => onFork(node.cat)}
            >
              <IconCopy size={14} stroke={1.5} />
            </ActionIcon>
          </Tooltip>
        ) : null}
        {isOwner && !node.cat.is_vault ? (
          <Tooltip label="Share category">
            <ActionIcon
              size="sm"
              variant="subtle"
              aria-label="Share category"
              onClick={() => onShare(node.cat)}
            >
              <IconShare size={14} stroke={1.5} />
            </ActionIcon>
          </Tooltip>
        ) : null}
        {isOwner ? (
          <Tooltip label="Delete category">
            <ActionIcon
              size="sm"
              variant="subtle"
              color="red"
              aria-label="Delete category"
              onClick={() => onDelete(node.cat.id)}
            >
              <IconTrash size={14} stroke={1.5} />
            </ActionIcon>
          </Tooltip>
        ) : null}
      </Group>
      {node.children.map((c) => (
        <CatRow
          key={c.cat.id}
          node={c}
          depth={depth + 1}
          onDelete={onDelete}
          onShare={onShare}
          onRename={onRename}
          onFork={onFork}
          forkPending={forkPending}
        />
      ))}
    </>
  )
}
