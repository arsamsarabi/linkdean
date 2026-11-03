import { createFileRoute } from '@tanstack/react-router'
import { ProfilePage } from '@/pages'

export type ProfileSearch = {
  /** Set by the API after a Connect from this page attached single sign-on. */
  sso?: 'connected'
  /** Why connecting single sign-on failed, set by the API's callback redirect. */
  oidc_error?: string
}

export const Route = createFileRoute('/_app/profile')({
  validateSearch: (search: Record<string, unknown>): ProfileSearch => ({
    sso: search.sso === 'connected' ? 'connected' : undefined,
    oidc_error:
      typeof search.oidc_error === 'string' && search.oidc_error.trim() ? search.oidc_error.trim() : undefined,
  }),
  component: ProfilePage,
})
