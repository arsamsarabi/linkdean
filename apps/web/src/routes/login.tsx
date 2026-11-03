import { createFileRoute } from '@tanstack/react-router'
import { LoginPage } from '@/pages'

export type LoginSearch = {
  /** Why an OIDC sign-in failed, set by the API's callback redirect. */
  oidc_error?: string
  /** A single sign-on is waiting for this account's password, set by the API's callback redirect. */
  link?: boolean
}

export const Route = createFileRoute('/login')({
  validateSearch: (search: Record<string, unknown>): LoginSearch => ({
    oidc_error:
      typeof search.oidc_error === 'string' && search.oidc_error.trim() ? search.oidc_error.trim() : undefined,
    link: search.link === 1 || search.link === '1' || search.link === true ? true : undefined,
  }),
  component: LoginPage,
})
