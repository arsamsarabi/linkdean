import { useMemo, useState, type ReactElement } from 'react'
import { Box, Button, Checkbox, Drawer, LoadingOverlay, Select, Stack, Text, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconDeviceFloppy } from '@tabler/icons-react'
import { useCategories, useSyncDuringRender, useUpdateCategory } from '@/hooks'
import { categorySelectData, categorySubtreeIds, isCategoryIconKey, type CategoryIconKey } from '@/lib'
import type { Category, CategoryVisibility } from '@/schemas'
import { CategoryIconPicker } from './CategoryIconPicker'
import { CategoryVisibilitySelect } from './CategoryVisibilitySelect'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  opened: boolean
  onClose: () => void
  category: Category | null
}

export function EditCategoryDrawer({ opened, onClose, category }: Props): ReactElement {
  const cats = useCategories()
  const update = useUpdateCategory()
  const [name, setName] = useState('')
  const [parentId, setParentId] = useState<string | null>(null)
  const [hidden, setHidden] = useState(false)
  const [visibility, setVisibility] = useState<CategoryVisibility>('private')
  const [icon, setIcon] = useState<CategoryIconKey | null>(null)

  const all = useMemo(() => cats.data ?? [], [cats.data])
  const isOwner = category?.access === 'owner'

  const parentOptions = useMemo(() => {
    if (!category) return [{ value: '', label: '- root -' }]
    const blocked = categorySubtreeIds(all, category.id)
    const candidates = all.filter((c) => !blocked.has(c.id) && (c.access === 'owner' || c.access === 'write'))
    return [{ value: '', label: '- root -' }, ...categorySelectData(candidates)]
  }, [all, category])

  useSyncDuringRender([opened, category], () => {
    if (!opened || !category) return
    setName(category.name)
    setParentId(category.parent_id ?? null)
    setHidden(category.is_hidden)
    setVisibility(category.visibility)
    setIcon(isCategoryIconKey(category.icon) ? category.icon : null)
  })

  async function onSave(): Promise<void> {
    if (!category) return
    const next = name.trim()
    if (!next) return

    const prevIcon = isCategoryIconKey(category.icon) ? category.icon : null
    const prevParent = category.parent_id ?? null
    const nameChanged = next !== category.name
    const iconChanged = icon !== prevIcon
    const parentChanged = parentId !== prevParent
    const hiddenChanged = isOwner && hidden !== category.is_hidden
    const visibilityChanged = isOwner && visibility !== category.visibility

    if (!nameChanged && !iconChanged && !parentChanged && !hiddenChanged && !visibilityChanged) {
      onClose()
      return
    }

    try {
      await update.mutateAsync({
        id: category.id,
        body: {
          ...(nameChanged ? { name: next } : {}),
          ...(iconChanged ? { icon } : {}),
          ...(parentChanged ? { parent_id: parentId } : {}),
          ...(hiddenChanged ? { is_hidden: hidden } : {}),
          ...(visibilityChanged ? { visibility } : {}),
        },
      })
      notifications.show({ message: 'Category updated' })
      onClose()
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Drawer opened={opened} onClose={onClose} title="Edit category" position="right" size="md" padding="lg">
      <Box pos="relative">
        <LoadingOverlay visible={update.isPending} zIndex={100} overlayProps={{ blur: 1 }} />
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
            onChange={(v) => setParentId(v || null)}
            searchable
            clearable
            disabled={!isOwner && category?.access !== 'write'}
          />
          <CategoryIconPicker
            value={icon}
            onChange={(next) => setIcon(next && isCategoryIconKey(next) ? next : null)}
          />
          {isOwner ? (
            <Checkbox label="Hidden" checked={hidden} onChange={(e) => setHidden(e.currentTarget.checked)} />
          ) : null}
          {isOwner ? (
            <CategoryVisibilitySelect value={visibility} isVault={!!category?.is_vault} onChange={setVisibility} />
          ) : null}
          {category?.is_vault ? (
            <Text size="sm" c="dimmed">
              Vault folder — encryption flag cannot be changed here.
            </Text>
          ) : null}
          <Button
            leftSection={<IconDeviceFloppy {...iconSm} />}
            onClick={() => void onSave()}
            disabled={!name.trim()}
            loading={update.isPending}
          >
            Save changes
          </Button>
        </Stack>
      </Box>
    </Drawer>
  )
}
