/** Terms of Service version shown at sign-up. Must equal `linkdean_core::legal::TERMS_VERSION`. */
export const TERMS_VERSION = '2026-11-11'

/** The API's single sign-on start URL. Sign-in may create an account, so it carries the accepted version. */
export function oidcStartUrl(provider?: string): string {
  const base = `/api/auth/oidc/start?terms_version=${encodeURIComponent(TERMS_VERSION)}`
  return provider ? `${base}&provider=${encodeURIComponent(provider)}` : base
}

/** Where Connect sends the browser: the provider, then back to the profile page. */
export function ssoConnectUrl(provider?: string): string {
  const base = '/api/auth/oidc/start?mode=connect'
  return provider ? `${base}&provider=${encodeURIComponent(provider)}` : base
}

/** Sign-in buttons to show: the server's list, or one generic button for older servers. */
export function ssoButtons(
  enabled: boolean,
  providers: readonly { id: string; label: string }[] | undefined
): { id: string; label: string }[] {
  if (providers && providers.length > 0) return [...providers]
  return enabled ? [{ id: 'oidc', label: 'single sign-on' }] : []
}

/** Signs in again with the attached single sign-on, for a step that needs a fresh sign-in. Returns to Settings. */
export const OIDC_REAUTH_URL = '/api/auth/oidc/start?mode=reauth'

/** The same, returning to the profile page (for adding a passkey). */
export const OIDC_REAUTH_PROFILE_URL = '/api/auth/oidc/start?mode=reauth&return=profile'

/** The API's refusal when a single sign-on only account must sign in again first. */
export const REAUTH_REQUIRED_MESSAGE = 'confirm it is you: sign in again with single sign-on'
