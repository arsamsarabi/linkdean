import { createFileRoute } from '@tanstack/react-router'
import { SettingsPage } from '@/pages'

export const Route = createFileRoute('/_app/settings')({
  component: SettingsPage,
  validateSearch: (search: Record<string, unknown>): { tab?: string; reauth?: string; oidc_error?: string } => ({
    tab: typeof search.tab === 'string' ? search.tab : undefined,
    reauth: search.reauth === 'done' ? 'done' : undefined,
    oidc_error:
      typeof search.oidc_error === 'string' && search.oidc_error.trim() ? search.oidc_error.trim() : undefined,
  }),
})
