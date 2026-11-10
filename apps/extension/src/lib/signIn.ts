import { browser } from 'wxt/browser'
import { z } from 'zod'
import { challengeFor, randomState, randomVerifier } from './pkce'
import { lastInstanceItem, sessionItem, type Session } from './session'

/** The address Linkdean sends the sign-in window back to, read from it by the browser. */
export function authorizeUrl(instance: string, redirectUri: string, challenge: string, state: string): string {
  const q = new URLSearchParams({
    redirect_uri: redirectUri,
    code_challenge: challenge,
    code_challenge_method: 'S256',
    state,
  })
  return `${instance}/connect/extension?${q.toString()}`
}

/** The code from the sign-in reply, once its state matches. */
export function codeFrom(responseUrl: string, state: string): string {
  const q = new URL(responseUrl).searchParams
  if (q.get('error')) throw new Error('Sign-in was cancelled.')
  if (q.get('state') !== state) throw new Error('The sign-in reply did not match. Try again.')
  const code = q.get('code')
  if (!code) throw new Error('Linkdean did not send a sign-in code. Try again.')
  return code
}

const tokenSchema = z.object({ token: z.string(), key_id: z.string(), email: z.string() })

/**
 * Signs in to `instance`: opens Linkdean's approve page in the browser's sign-in window, then
 * trades the code it returns, with the PKCE verifier only this extension holds, for a key.
 * Runs in the background worker, so closing the popup does not end it.
 */
export async function signIn(instance: string): Promise<Session> {
  const verifier = randomVerifier()
  const state = randomState()
  const redirectUri = browser.identity.getRedirectURL()
  const url = authorizeUrl(instance, redirectUri, await challengeFor(verifier), state)
  let responseUrl: string | undefined
  try {
    responseUrl = await browser.identity.launchWebAuthFlow({ url, interactive: true })
  } catch {
    throw new Error('Sign-in was cancelled, or this Linkdean does not support the extension yet.')
  }
  if (!responseUrl) throw new Error('Sign-in was cancelled.')
  const code = codeFrom(responseUrl, state)
  const res = await fetch(`${instance}/api/auth/extension/token`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code, code_verifier: verifier, redirect_uri: redirectUri }),
  })
  const body: unknown = await res.json().catch(() => null)
  if (!res.ok) {
    const msg =
      body && typeof body === 'object' && 'error' in body ? String(body.error) : `Linkdean answered ${res.status}.`
    throw new Error(msg)
  }
  const t = tokenSchema.parse(body)
  const session: Session = { instance, token: t.token, keyId: t.key_id, email: t.email }
  await sessionItem.setValue(session)
  await lastInstanceItem.setValue(instance)
  return session
}
