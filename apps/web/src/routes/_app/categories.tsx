import { createFileRoute } from '@tanstack/react-router'
import { CategoriesPage } from '@/pages'

export const Route = createFileRoute('/_app/categories')({
  component: CategoriesPage,
  validateSearch: (search: Record<string, unknown>): { id?: string } => ({
    id: typeof search.id === 'string' ? search.id : undefined,
  }),
})
