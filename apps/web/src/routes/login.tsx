import { createFileRoute } from '@tanstack/react-router'
import { LoginPage } from '@/pages'

export type LoginSearch = {
  /** Why an OIDC sign-in failed, set by the API's callback redirect. */
  oidc_error?: string
}

export const Route = createFileRoute('/login')({
  validateSearch: (search: Record<string, unknown>): LoginSearch => ({
    oidc_error:
      typeof search.oidc_error === 'string' && search.oidc_error.trim() ? search.oidc_error.trim() : undefined,
  }),
  component: LoginPage,
})
