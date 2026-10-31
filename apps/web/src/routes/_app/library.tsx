import { createFileRoute } from '@tanstack/react-router'
import { validateLibrarySearch } from '@/lib'
import { LibraryPage } from '@/pages'

export const Route = createFileRoute('/_app/library')({
  component: LibraryPage,
  validateSearch: (search: Record<string, unknown>) => validateLibrarySearch(search),
})
