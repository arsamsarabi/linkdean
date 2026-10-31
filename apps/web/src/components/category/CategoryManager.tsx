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
import { IconFolder, IconFolderPlus, IconTrash } from '@tabler/icons-react'
import {
  useCategories,
  useCreateCategory,
  useDeleteCategory,
  useUpdateCategory,
} from '@/hooks'
import { buildCategoryTree, categorySelectData, type CatNode } from '@/lib'
import { ConfirmModal } from '@/components/confirm-modal'
import { QueryErrorAlert } from '@/components/query-error-alert'

const iconSm = { size: 16, stroke: 1.5 } as const

export function CategoryManager(): ReactElement {
  const [name, setName] = useState('')
  const [parentId, setParentId] = useState<string | null>(null)
  const [hidden, setHidden] = useState(false)
  const [vault, setVault] = useState(false)
  const [deleteId, setDeleteId] = useState<string | null>(null)

  const cats = useCategories(true)
  const create = useCreateCategory()
  const update = useUpdateCategory()
  const remove = useDeleteCategory()

  const tree = useMemo(() => buildCategoryTree(cats.data || []), [cats.data])
  const parentOptions = useMemo(
    () => [{ value: '', label: '— root —' }, ...categorySelectData(cats.data || [])],
    [cats.data],
  )

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

  return (
    <Stack gap="lg">
      <Text size="sm" c="dimmed">
        Nested folders for the library. Manage here — library filters by category.
      </Text>

      {cats.isError ? (
        <QueryErrorAlert error={cats.error} onRetry={() => void cats.refetch()} />
      ) : null}

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
                  try {
                    await update.mutateAsync({ id, body: { name: next } })
                  } catch (e) {
                    notifications.show({ color: 'red', message: String(e) })
                  }
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
    </Stack>
  )
}

type CatRowProps = {
  node: CatNode
  depth: number
  onDelete: (id: string) => void
  onRename: (id: string, name: string) => Promise<void>
}

function CatRow({ node, depth, onDelete, onRename }: CatRowProps): ReactElement {
  const [editing, setEditing] = useState(false)
  const [name, setName] = useState(node.cat.name)

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
        {editing ? (
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
        ) : (
          <UnstyledButton
            type="button"
            onClick={() => setEditing(true)}
            aria-label={`Rename ${node.cat.name}`}
            style={{ flex: 1, textAlign: 'left' }}
          >
            <Text size="sm">{node.cat.name}</Text>
          </UnstyledButton>
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
