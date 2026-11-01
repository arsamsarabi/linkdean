import { createFileRoute } from '@tanstack/react-router'
import { validateLibrarySearch } from '@/lib'
import { VaultsPage } from '@/pages'

export const Route = createFileRoute('/_app/vaults')({
  component: VaultsPage,
  validateSearch: (search: Record<string, unknown>) => validateLibrarySearch(search),
})
