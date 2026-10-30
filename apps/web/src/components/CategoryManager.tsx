import { useMemo, useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
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
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconFolder, IconFolderPlus, IconTrash } from '@tabler/icons-react'
import { api } from '../api'
import { ConfirmModal } from './ConfirmModal'
import { buildCategoryTree, categorySelectData, type CatNode } from '../lib/categories'
import { useUi } from '../store'

const iconSm = { size: 16, stroke: 1.5 } as const

export function CategoryManager() {
  const qc = useQueryClient()
  const vaultUnlocked = useUi((s) => s.vaultUnlocked)
  const [name, setName] = useState('')
  const [parentId, setParentId] = useState<string | null>(null)
  const [hidden, setHidden] = useState(false)
  const [vault, setVault] = useState(false)
  const [deleteId, setDeleteId] = useState<string | null>(null)
  const [deleting, setDeleting] = useState(false)

  const cats = useQuery({
    queryKey: ['categories', true, vaultUnlocked],
    queryFn: () => api.categories(true, vaultUnlocked),
  })

  const tree = useMemo(() => buildCategoryTree(cats.data || []), [cats.data])
  const parentOptions = useMemo(
    () => [{ value: '', label: '— root —' }, ...categorySelectData(cats.data || [])],
    [cats.data],
  )

  const create = useMutation({
    mutationFn: () =>
      api.createCategory({
        name: name.trim(),
        parent_id: parentId || null,
        is_hidden: hidden,
        is_vault: vault,
      }),
    onSuccess: () => {
      setName('')
      setParentId(null)
      setHidden(false)
      setVault(false)
      qc.invalidateQueries({ queryKey: ['categories'] })
      notifications.show({ message: 'Category created' })
    },
    onError: (e) => notifications.show({ color: 'red', message: String(e) }),
  })

  async function confirmDelete() {
    if (!deleteId) return
    setDeleting(true)
    try {
      await api.deleteCategory(deleteId)
      qc.invalidateQueries({ queryKey: ['categories'] })
      qc.invalidateQueries({ queryKey: ['bookmarks'] })
      notifications.show({ message: 'Category deleted' })
      setDeleteId(null)
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    } finally {
      setDeleting(false)
    }
  }

  return (
    <Stack gap="lg">
      <Text size="sm" c="dimmed">
        Nested folders for the library. Manage here — library filters by category.
      </Text>

      {cats.isLoading ? (
        <Stack gap="xs">
          <Skeleton height={28} />
          <Skeleton height={28} width="80%" />
          <Skeleton height={28} width="60%" />
        </Stack>
      ) : (
        <ScrollArea.Autosize mah={320} type="auto" offsetScrollbars>
          <Stack gap={6}>
            {tree.length === 0 ? <Text c="dimmed">No categories yet.</Text> : null}
            {tree.map((n) => (
              <CatRow
                key={n.cat.id}
                node={n}
                depth={0}
                onDelete={setDeleteId}
                onRename={async (id, next) => {
                  await api.updateCategory(id, { name: next })
                  qc.invalidateQueries({ queryKey: ['categories'] })
                  qc.invalidateQueries({ queryKey: ['bookmarks'] })
                }}
              />
            ))}
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
          onChange={(v) => setParentId(v || null)}
          searchable
          clearable
        />
        <Group gap="lg">
          <Checkbox
            label="Hidden"
            checked={hidden}
            onChange={(e) => setHidden(e.currentTarget.checked)}
          />
          <Checkbox
            label="Vault"
            checked={vault}
            onChange={(e) => setVault(e.currentTarget.checked)}
          />
        </Group>
        <Button
          leftSection={<IconFolderPlus {...iconSm} />}
          onClick={() => create.mutate()}
          disabled={!name.trim()}
          loading={create.isPending}
        >
          Add category
        </Button>
      </Stack>

      <ConfirmModal
        opened={!!deleteId}
        onClose={() => setDeleteId(null)}
        onConfirm={confirmDelete}
        title="Delete category?"
        body="Bookmarks in this category keep their links but lose this folder assignment."
        confirmLabel="Delete"
        loading={deleting}
      />
    </Stack>
  )
}

function CatRow({
  node,
  depth,
  onDelete,
  onRename,
}: {
  node: CatNode
  depth: number
  onDelete: (id: string) => void
  onRename: (id: string, name: string) => Promise<void>
}) {
  const [editing, setEditing] = useState(false)
  const [name, setName] = useState(node.cat.name)

  return (
    <>
      <Group gap="xs" wrap="nowrap" style={{ paddingLeft: depth * 16 }}>
        <IconFolder size={14} stroke={1.5} style={{ flexShrink: 0 }} />
        {editing ? (
          <TextInput
            size="xs"
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            onBlur={async () => {
              setEditing(false)
              if (name.trim() && name.trim() !== node.cat.name) {
                await onRename(node.cat.id, name.trim())
              } else {
                setName(node.cat.name)
              }
            }}
            onKeyDown={async (e) => {
              if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
              if (e.key === 'Escape') {
                setName(node.cat.name)
                setEditing(false)
              }
            }}
            style={{ flex: 1 }}
            autoFocus
          />
        ) : (
          <Text
            size="sm"
            style={{ flex: 1, cursor: 'pointer' }}
            onClick={() => setEditing(true)}
            title="Click to rename"
          >
            {node.cat.name}
          </Text>
        )}
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
      </Group>
      {node.children.map((c) => (
        <CatRow
          key={c.cat.id}
          node={c}
          depth={depth + 1}
          onDelete={onDelete}
          onRename={onRename}
        />
      ))}
    </>
  )
}
