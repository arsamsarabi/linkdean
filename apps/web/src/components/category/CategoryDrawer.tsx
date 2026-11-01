import { useMemo, useState, type ReactElement } from 'react'
import { Box, Button, Checkbox, Drawer, LoadingOverlay, Select, Stack, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconFolderPlus } from '@tabler/icons-react'
import { useCreateCategory, useSyncDuringRender, useVaults } from '@/hooks'
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
  const [vaultId, setVaultId] = useState<string | null>(null)
  const vaults = useVaults(opened)
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
  const canChooseVault = !parentCat

  useSyncDuringRender([opened, defaultParentId], () => {
    if (!opened) return
    setName('')
    setParentId(defaultParentId)
    setHidden(false)
    setVaultId(null)
    setIcon(null)
  })

  async function onCreate(): Promise<void> {
    try {
      await create.mutateAsync({
        name: name.trim(),
        parent_id: parentId || null,
        is_hidden: hidden,
        vault_id: parentCat ? (parentCat.vault_id ?? null) : vaultId,
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
              setParentId(v || null)
              setVaultId(null)
            }}
            searchable
            clearable
          />
          <CategoryIconPicker
            value={icon}
            onChange={(next) => setIcon(next && isCategoryIconKey(next) ? next : null)}
          />
          <Checkbox label="Hidden" checked={hidden} onChange={(e) => setHidden(e.currentTarget.checked)} />
          <Select
            label="Vault"
            placeholder={parentCat ? 'Follows the parent category' : 'Not in a vault'}
            clearable
            disabled={!canChooseVault}
            data={(vaults.data?.vaults ?? []).filter((v) => v.unlocked).map((v) => ({ value: v.id, label: v.name }))}
            value={vaultId}
            onChange={setVaultId}
          />
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
