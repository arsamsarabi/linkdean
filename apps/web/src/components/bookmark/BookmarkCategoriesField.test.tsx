import { useState, type ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { categorySchema, type Category } from '@/schemas'
import { BookmarkCategoriesField } from './BookmarkCategoriesField'

const createCategory = { isPending: false, mutateAsync: vi.fn() }

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useCreateCategory: () => createCategory,
}))

vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))

function cat(id: string, name: string, extra: Partial<Category> = {}): Category {
  return categorySchema.parse({ id, name, is_hidden: false, is_vault: false, position: 0, ...extra })
}

const reading = cat('c1', 'Reading')
const work = cat('c2', 'Work')

let selected: string[] = []

function Harness({ categories, canCreate = true }: { categories: Category[]; canCreate?: boolean }): ReactElement {
  const [value, setValue] = useState<string[]>([])
  return (
    <MantineProvider env="test">
      <BookmarkCategoriesField
        categories={categories}
        value={value}
        onChange={(ids) => {
          selected = ids
          setValue(ids)
        }}
        canCreate={canCreate}
      />
    </MantineProvider>
  )
}

beforeAll(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})

beforeEach(() => {
  selected = []
  createCategory.mutateAsync.mockReset()
})

afterEach(cleanup)

function picker(): HTMLElement {
  return screen.getAllByLabelText('Categories').find((el) => el.tagName === 'INPUT')!
}

describe('BookmarkCategoriesField', () => {
  it('offers to create a category whose name matches nothing', async () => {
    render(<Harness categories={[reading, work]} />)
    await userEvent.type(picker(), 'Recipes')
    expect(await screen.findByRole('option', { name: 'Create "Recipes"' })).toBeTruthy()
  })

  it('does not offer to create a name that already exists, whatever the case', async () => {
    render(<Harness categories={[reading, work]} />)
    await userEvent.type(picker(), 'reading')
    expect(await screen.findByRole('option', { name: 'Reading' })).toBeTruthy()
    expect(screen.queryByRole('option', { name: /^Create/ })).toBeNull()
  })

  it('does not offer to create when the bookmark is read-only', async () => {
    render(<Harness categories={[reading]} canCreate={false} />)
    await userEvent.type(picker(), 'Recipes')
    await waitFor(() => expect(screen.queryByRole('option', { name: /^Create/ })).toBeNull())
  })

  it('creates the category at the top level, selects it, and toasts Category created', async () => {
    const { notifications } = await import('@mantine/notifications')
    createCategory.mutateAsync.mockResolvedValue(cat('new1', 'Recipes'))
    render(<Harness categories={[reading]} />)
    await userEvent.type(picker(), 'Recipes')
    await userEvent.click(await screen.findByRole('option', { name: 'Create "Recipes"' }))

    expect(createCategory.mutateAsync).toHaveBeenCalledWith({
      name: 'Recipes',
      parent_id: null,
      is_hidden: false,
      vault_id: null,
    })
    await waitFor(() => expect(selected).toEqual(['new1']))
    // The new category shows as a chosen pill even before the category list refetches.
    expect(Array.from(document.querySelectorAll('.mantine-Pill-label'), (el) => el.textContent)).toEqual(['Recipes'])
    expect(notifications.show).toHaveBeenCalledWith({ message: 'Category created' })
  })

  it('selects nothing when creating fails', async () => {
    createCategory.mutateAsync.mockRejectedValue(new Error('Name already used'))
    render(<Harness categories={[reading]} />)
    await userEvent.type(picker(), 'Recipes')
    await userEvent.click(await screen.findByRole('option', { name: 'Create "Recipes"' }))

    await waitFor(() => expect(createCategory.mutateAsync).toHaveBeenCalled())
    expect(selected).toEqual([])
    expect(screen.queryByRole('textbox', { name: 'New category' })).toBeNull()
  })
})
