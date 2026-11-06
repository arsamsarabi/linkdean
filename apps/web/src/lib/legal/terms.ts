/** Terms of Service version shown at sign-up. Must equal `linkdean_core::legal::TERMS_VERSION`. */
export const TERMS_VERSION = '2026-11-06'

/** The API's OIDC sign-in start URL. Sign-in may create an account, so it carries the accepted version. */
export function oidcStartUrl(): string {
  return `/api/auth/oidc/start?terms_version=${encodeURIComponent(TERMS_VERSION)}`
}
