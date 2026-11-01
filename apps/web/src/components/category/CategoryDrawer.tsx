import { useMemo, useState, type ReactElement } from 'react'
import { Box, Button, Checkbox, Drawer, Group, LoadingOverlay, Select, Stack, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconFolderPlus } from '@tabler/icons-react'
import { useCreateCategory, useSyncDuringRender } from '@/hooks'
import { categorySelectData, isCategoryIconKey, type CategoryIconKey } from '@/lib'
import type { Category } from '@/schemas'
import { CategoryIconPicker } from './CategoryIconPicker'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  opened: boolean
  onClose: () => void
  categories: Category[]
  /** Prefill parent when opening from a folder. */
  defaultParentId?: string | null
}

export function CategoryDrawer({ opened, onClose, categories, defaultParentId = null }: Props): ReactElement {
  const [name, setName] = useState('')
  const [parentId, setParentId] = useState<string | null>(null)
  const [hidden, setHidden] = useState(false)
  const [vault, setVault] = useState(false)
  const [icon, setIcon] = useState<CategoryIconKey | null>(null)
  const create = useCreateCategory()

  const parentOptions = useMemo(
    () => [
      { value: '', label: '- root -' },
      ...categorySelectData(categories.filter((c) => c.access === 'owner' || c.access === 'write')),
    ],
    [categories]
  )

  const parentCat = useMemo(() => (parentId ? categories.find((c) => c.id === parentId) : null), [categories, parentId])
  const canCreateVault = !parentCat || parentCat.access === 'owner'

  useSyncDuringRender([opened, defaultParentId], () => {
    if (!opened) return
    setName('')
    setParentId(defaultParentId)
    setHidden(false)
    setVault(false)
    setIcon(null)
  })

  async function onCreate(): Promise<void> {
    try {
      await create.mutateAsync({
        name: name.trim(),
        parent_id: parentId || null,
        is_hidden: hidden,
        is_vault: vault,
        icon: icon ?? null,
      })
      notifications.show({ message: 'Category created' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Drawer opened={opened} onClose={onClose} title="New category" position="right" size="md" padding="lg">
      <Box pos="relative">
        <LoadingOverlay visible={create.isPending} zIndex={100} overlayProps={{ blur: 1 }} />
        <Stack gap="md">
          <TextInput
            label="Name"
            placeholder="Name"
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            data-autofocus
          />
          <Select
            label="Parent"
            data={parentOptions}
            value={parentId ?? ''}
            onChange={(v) => {
              const next = v || null
              setParentId(next)
              const p = next ? categories.find((c) => c.id === next) : null
              if (p && p.access !== 'owner') setVault(false)
            }}
            searchable
            clearable
          />
          <CategoryIconPicker
            value={icon}
            onChange={(next) => setIcon(next && isCategoryIconKey(next) ? next : null)}
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
      </Box>
    </Drawer>
  )
}
