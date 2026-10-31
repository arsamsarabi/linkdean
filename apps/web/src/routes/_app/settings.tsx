import { createFileRoute } from '@tanstack/react-router'
import { SettingsPage } from '@/pages'

export const Route = createFileRoute('/_app/settings')({
  component: SettingsPage,
  validateSearch: (search: Record<string, unknown>): { tab?: string } => ({
    tab: typeof search.tab === 'string' ? search.tab : undefined,
  }),
})
