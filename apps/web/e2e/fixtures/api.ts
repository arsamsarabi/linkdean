import type { APIRequestContext, APIResponse } from '@playwright/test'

/** Header the API requires on cookie-authenticated writes (see src/lib/auth/authToken.ts). */
export const CLIENT_HEADER = { 'X-Linkdean-Client': 'web' }

export const E2E_PASSWORD = 'e2e-correct-horse-battery'

const START_API_HINT =
  'Start the throwaway stack first (API on BIND=127.0.0.1:18081, Vite on :5199); see apps/web/e2e/README.md.'

/**
 * Runs one API call and throws on a non-2xx. A network error, or an empty 5xx (what Vite's proxy
 * returns when nothing listens on the API port), means the API is down, so say how to start it.
 */
async function call(what: string, fn: () => Promise<APIResponse>): Promise<APIResponse> {
  let res: APIResponse
  try {
    res = await fn()
  } catch (e) {
    throw new Error(`${what}: cannot reach the API. ${START_API_HINT} (${e instanceof Error ? e.message : e})`)
  }
  if (res.ok()) return res
  const body = await res.text()
  if (res.status() >= 500 && body.trim() === '') {
    throw new Error(`${what}: API unreachable (HTTP ${res.status()}, empty body). ${START_API_HINT}`)
  }
  throw new Error(`${what} failed: ${res.status()} ${body}`)
}

/** `terms_version` from GET /api/auth/bootstrap-status; '' when the server sets none. */
export async function getTermsVersion(request: APIRequestContext): Promise<string> {
  const res = await call('GET /api/auth/bootstrap-status', () => request.get('/api/auth/bootstrap-status'))
  const body = (await res.json()) as { terms_version?: string | null }
  return body.terms_version ?? ''
}

/** Registers a local user; the response sets the `linkdean_session` cookie on `request`. */
export async function registerUser(
  request: APIRequestContext,
  user: { email: string; password?: string; displayName?: string; bootstrapToken?: string }
): Promise<void> {
  const terms_version = await getTermsVersion(request)
  await call('POST /api/auth/register', () =>
    request.post('/api/auth/register', {
      headers: CLIENT_HEADER,
      data: {
        email: user.email,
        password: user.password ?? E2E_PASSWORD,
        display_name: user.displayName,
        bootstrap_token: user.bootstrapToken ?? process.env.E2E_BOOTSTRAP_TOKEN,
        terms_version,
      },
    })
  )
}

/** Password sign-in; the response sets the `linkdean_session` cookie on `request`. */
export async function loginRequest(
  request: APIRequestContext,
  email: string,
  password: string = E2E_PASSWORD
): Promise<void> {
  await call('POST /api/auth/login', () =>
    request.post('/api/auth/login', { headers: CLIENT_HEADER, data: { email, password } })
  )
}

/** Marks onboarding done for the cookie-authenticated user on `request`. */
export async function completeOnboarding(request: APIRequestContext): Promise<void> {
  await call('POST /api/auth/onboarding/complete', () =>
    request.post('/api/auth/onboarding/complete', { headers: CLIENT_HEADER })
  )
}
