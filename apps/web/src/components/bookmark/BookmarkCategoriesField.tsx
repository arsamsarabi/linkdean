import { useMemo, useState, type ReactElement } from 'react'
import { MultiSelect, Stack } from '@mantine/core'
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
 * **Create "name"**, which creates it at the top level and selects it.
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

  const typed = search.trim()
  const nameTaken = writable.some((c) => c.name.toLowerCase() === typed.toLowerCase())
  const offerCreate = canCreate && typed !== '' && !nameTaken
  const data = offerCreate ? [...options, { value: CREATE_OPTION, label: `Create "${typed}"` }] : options

  async function createNamed(name: string): Promise<void> {
    const trimmed = name.trim()
    if (!trimmed) return
    try {
      const cat = await createCategory.mutateAsync({
        name: trimmed,
        parent_id: null,
        is_hidden: false,
        vault_id: null,
      })
      setCreated((prev) => [...prev, cat])
      onChange([...value, cat.id])
      notifications.show({ message: 'Category created' })
    } catch (e) {
      notifications.show({ color: 'red', message: e instanceof Error ? e.message : String(e) })
    }
  }

  function handleChange(ids: string[]): void {
    if (ids.includes(CREATE_OPTION)) {
      const name = typed
      setSearch('')
      void createNamed(name)
      return
    }
    onChange(ids)
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
        disabled={createCategory.isPending}
      />
    </Stack>
  )
}
