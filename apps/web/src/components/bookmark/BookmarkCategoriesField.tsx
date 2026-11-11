import { useMemo, useState, type ReactElement } from 'react'
import { Button, Group, MultiSelect, Paper, Select, Stack, TextInput } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { useCreateCategory } from '@/hooks'
import { categorySelectData, writableCategories } from '@/lib'
import type { Category } from '@/schemas'

/** Option value for the "Create …" row; never a real category id. */
const CREATE_OPTION = '__create_category__'

type Props = {
  categories: Category[]
  value: string[]
  onChange: (categoryIds: string[]) => void
  /** Offer to create a category from what was typed. Off for bookmarks the person cannot edit. */
  canCreate: boolean
}

/**
 * The bookmark form's Categories picker. Typing a name that matches no category offers
 * **Create "name"**, which opens a small panel to pick where it goes (top level by default),
 * then creates the category and selects it.
 */
export function BookmarkCategoriesField({ categories, value, onChange, canCreate }: Props): ReactElement {
  const createCategory = useCreateCategory()
  // Categories made here, shown until the refetched list includes them.
  const [created, setCreated] = useState<Category[]>([])
  const writable = useMemo(() => {
    const list = writableCategories(categories)
    const known = new Set(list.map((c) => c.id))
    return [...list, ...created.filter((c) => !known.has(c.id))]
  }, [categories, created])
  const options = useMemo(() => categorySelectData(writable), [writable])

  const [search, setSearch] = useState('')
  const [newName, setNewName] = useState<string | null>(null)
  const [parentId, setParentId] = useState<string | null>(null)

  const typed = search.trim()
  const nameTaken = writable.some((c) => c.name.toLowerCase() === typed.toLowerCase())
  const offerCreate = canCreate && newName === null && typed !== '' && !nameTaken
  const data = offerCreate ? [...options, { value: CREATE_OPTION, label: `Create "${typed}"` }] : options

  function handleChange(ids: string[]): void {
    if (ids.includes(CREATE_OPTION)) {
      setNewName(typed)
      setParentId(null)
      setSearch('')
      return
    }
    onChange(ids)
  }

  async function submit(): Promise<void> {
    const name = newName?.trim()
    if (!name) return
    const parent = parentId ? writable.find((c) => c.id === parentId) : undefined
    try {
      const cat = await createCategory.mutateAsync({
        name,
        parent_id: parent?.id ?? null,
        // A subcategory lives in its parent's vault, as in the New category drawer.
        vault_id: parent?.vault_id ?? null,
      })
      setCreated((prev) => [...prev, cat])
      onChange([...value, cat.id])
      setNewName(null)
      notifications.show({ message: `Created "${cat.name}"` })
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  return (
    <Stack gap="xs">
      <MultiSelect
        label="Categories"
        description={
          canCreate
            ? 'Can pick several. Type a new name to create one. Read-only shared folders are omitted.'
            : 'Can pick several. Nested path shown in label. Read-only shared folders are omitted.'
        }
        data={data}
        value={value}
        onChange={handleChange}
        searchValue={search}
        onSearchChange={setSearch}
        searchable
        clearable
      />
      {newName !== null ? (
        <Paper withBorder p="sm" radius="md">
          <Stack gap="xs">
            <TextInput
              label="New category"
              value={newName}
              onChange={(e) => {
                const next = e.currentTarget.value
                setNewName(next)
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault()
                  void submit()
                }
              }}
              autoFocus
            />
            <Select
              label="Inside"
              placeholder="Top level"
              data={options}
              value={parentId}
              onChange={setParentId}
              searchable
              clearable
            />
            <Group justify="flex-end" gap="xs">
              <Button variant="default" size="xs" onClick={() => setNewName(null)} disabled={createCategory.isPending}>
                Cancel
              </Button>
              <Button
                size="xs"
                onClick={() => void submit()}
                disabled={!newName.trim()}
                loading={createCategory.isPending}
              >
                Create and select
              </Button>
            </Group>
          </Stack>
        </Paper>
      ) : null}
    </Stack>
  )
}
