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
const vaultParent = cat('c3', 'Secrets', { is_vault: true, vault_id: 'v1' })
const readOnly = cat('c4', 'Their folder', { access: 'read' })

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

function input(label: string): HTMLElement {
  return screen.getAllByLabelText(label).find((el) => el.tagName === 'INPUT')!
}

function picker(): HTMLElement {
  return input('Categories')
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

  it('creates the category at the top level and selects it', async () => {
    createCategory.mutateAsync.mockResolvedValue(cat('new1', 'Recipes'))
    render(<Harness categories={[reading]} />)
    await userEvent.type(picker(), 'Recipes')
    await userEvent.click(await screen.findByRole('option', { name: 'Create "Recipes"' }))

    expect((screen.getByRole('textbox', { name: 'New category' }) as HTMLInputElement).value).toBe('Recipes')
    await userEvent.click(screen.getByRole('button', { name: 'Create and select' }))

    expect(createCategory.mutateAsync).toHaveBeenCalledWith({
      name: 'Recipes',
      parent_id: null,
      is_hidden: false,
      vault_id: null,
    })
    await waitFor(() => expect(selected).toEqual(['new1']))
    expect(screen.queryByRole('textbox', { name: 'New category' })).toBeNull()
    // The new category shows as a chosen pill even before the category list refetches.
    expect(Array.from(document.querySelectorAll('.mantine-Pill-label'), (el) => el.textContent)).toEqual(['Recipes'])
  })

  it('creates under a chosen parent, inheriting its vault', async () => {
    createCategory.mutateAsync.mockResolvedValue(cat('new2', 'Keys', { parent_id: 'c3', vault_id: 'v1' }))
    render(<Harness categories={[reading, vaultParent]} />)
    await userEvent.type(picker(), 'Keys')
    await userEvent.click(await screen.findByRole('option', { name: 'Create "Keys"' }))
    await userEvent.click(input('Inside'))
    await userEvent.click(await screen.findByRole('option', { name: 'Secrets (vault)' }))
    await userEvent.click(screen.getByRole('button', { name: 'Create and select' }))

    expect(createCategory.mutateAsync).toHaveBeenCalledWith({
      name: 'Keys',
      parent_id: 'c3',
      is_hidden: false,
      vault_id: 'v1',
    })
    await waitFor(() => expect(selected).toEqual(['new2']))
  })

  it('offers only writable folders as parents', async () => {
    render(<Harness categories={[reading, readOnly]} />)
    await userEvent.type(picker(), 'Recipes')
    await userEvent.click(await screen.findByRole('option', { name: 'Create "Recipes"' }))
    await userEvent.click(input('Inside'))
    expect(await screen.findByRole('option', { name: 'Reading' })).toBeTruthy()
    expect(screen.queryByRole('option', { name: 'Their folder' })).toBeNull()
  })

  it('keeps the panel open and selects nothing when creating fails', async () => {
    createCategory.mutateAsync.mockRejectedValue(new Error('Name already used'))
    render(<Harness categories={[reading]} />)
    await userEvent.type(picker(), 'Recipes')
    await userEvent.click(await screen.findByRole('option', { name: 'Create "Recipes"' }))
    await userEvent.click(screen.getByRole('button', { name: 'Create and select' }))

    await waitFor(() => expect(createCategory.mutateAsync).toHaveBeenCalled())
    expect(screen.getByRole('textbox', { name: 'New category' })).toBeTruthy()
    expect(selected).toEqual([])
  })
})
