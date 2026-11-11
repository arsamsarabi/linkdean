import { createFileRoute } from '@tanstack/react-router'
import { AdminPage } from '@/pages'

export const Route = createFileRoute('/_app/admin')({
  component: AdminPage,
  validateSearch: (search: Record<string, unknown>): { tab?: string } => ({
    tab: typeof search.tab === 'string' ? search.tab : undefined,
  }),
})
