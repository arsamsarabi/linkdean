/** Terms of Service version shown at sign-up. Must equal `linkdean_core::legal::TERMS_VERSION`. */
export const TERMS_VERSION = '2026-11-11'

/** The API's OIDC sign-in start URL. Sign-in may create an account, so it carries the accepted version. */
export function oidcStartUrl(): string {
  return `/api/auth/oidc/start?terms_version=${encodeURIComponent(TERMS_VERSION)}`
}

/** Signs in again with the attached single sign-on, for a step that needs a fresh sign-in. Returns to Settings. */
export const OIDC_REAUTH_URL = '/api/auth/oidc/start?mode=reauth'

/** The API's refusal when a single sign-on only account must sign in again first. */
export const REAUTH_REQUIRED_MESSAGE = 'confirm it is you: sign in again with single sign-on'
