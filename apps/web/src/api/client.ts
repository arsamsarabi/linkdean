import type { z } from 'zod'
import { CLIENT_HEADER, isSignedIn, markSignedIn } from '@/lib'
import { banNoticeSchema, type BanNotice } from '@/schemas'

function parseWithSchema<T>(schema: z.ZodType<T>, data: unknown, path: string): T {
  const result = schema.safeParse(data)
  if (!result.success) {
    const detail = result.error.issues
      .slice(0, 3)
      .map((i) => `${i.path.join('.') || '(root)'}: ${i.message}`)
      .join('; ')
    throw new Error(`Invalid API response for ${path}: ${detail}`)
  }
  return result.data
}

export class ApiError extends Error {
  readonly status: number
  readonly body: unknown

  constructor(message: string, status: number, body: unknown) {
    super(message)
    this.status = status
    this.body = body
  }
}

/** The ban in a 403 body, or null for anything else. */
export function banFromBody(body: unknown): BanNotice | null {
  if (!body || typeof body !== 'object' || (body as { code?: unknown }).code !== 'banned') return null
  return banNoticeSchema.safeParse((body as { ban?: unknown }).ban).data ?? null
}

/** The hard gate's 403: this account must verify its email before using the app. */
export function isEmailUnverified(body: unknown): boolean {
  return !!body && typeof body === 'object' && (body as { code?: unknown }).code === 'email_unverified'
}

/** A wrong password is also a 401, so only this code means the session has gone. */
function sessionExpired(body: unknown): boolean {
  return !!body && typeof body === 'object' && (body as { code?: unknown }).code === 'session_expired'
}

/** Pages a signed-out visitor may stay on. Elsewhere a cookie session (OIDC) has no token to clear. */
function isPublicPath(pathname: string): boolean {
  return (
    pathname === '/login' ||
    pathname === '/register' ||
    pathname === '/onboarding' ||
    pathname === '/banned' ||
    pathname === '/forgot-password' ||
    pathname === '/reset-password' ||
    pathname === '/verify-email' ||
    pathname.startsWith('/l/')
  )
}

/**
 * Pages that show their own signed-out state instead of being sent to login. `/verify-email` has
 * already spent its link by the time it asks about the session, so it shows the result with a
 * Sign in button.
 */
function handlesSignOut(pathname: string): boolean {
  return pathname === '/login' || pathname === '/verify-email'
}

export async function req<T>(path: string, init: RequestInit = {}, schema?: z.ZodType<T>): Promise<T> {
  const headers = new Headers(init.headers)
  if (!headers.has('Content-Type') && init.body && !(init.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json')
  }
  // The session rides in the HttpOnly cookie; this header marks the request as the app's own.
  headers.set(CLIENT_HEADER, 'web')
  const res = await fetch(`/api${path}`, { ...init, headers, credentials: 'include' })
  if (!res.ok) {
    let msg = res.statusText
    let body: unknown
    try {
      body = await res.json()
      if (
        body &&
        typeof body === 'object' &&
        'error' in body &&
        typeof (body as { error: unknown }).error === 'string'
      ) {
        msg = (body as { error: string }).error
      }
    } catch {
      /* ignore body parse */
    }
    if (res.status === 403 && banFromBody(body) && location.pathname !== '/banned') {
      location.assign('/banned')
    }
    if (res.status === 403 && isEmailUnverified(body) && location.pathname !== '/verify-email') {
      location.assign('/verify-email')
    }
    if (res.status === 401 && sessionExpired(body) && (isSignedIn() || !isPublicPath(location.pathname))) {
      // A full page load also drops the signed-out user's cached data.
      markSignedIn(false)
      if (!handlesSignOut(location.pathname)) location.assign('/login')
    }
    throw new ApiError(msg, res.status, body)
  }
  if (res.status === 204) {
    if (schema) return parseWithSchema(schema, undefined, path)
    return undefined as T
  }
  const ct = res.headers.get('content-type') || ''
  if (ct.includes('application/json')) {
    const json: unknown = await res.json()
    if (schema) return parseWithSchema(schema, json, path)
    return json as T
  }
  const text = await res.text()
  if (schema) return parseWithSchema(schema, text, path)
  return text as T
}

/** Stable avatar URL; pass bust only when intentionally cache-busting. */
export function avatarUrl(hasAvatar: boolean, bust?: number | string): string | undefined {
  if (!hasAvatar) return undefined
  const q = bust != null ? `?v=${bust}` : ''
  return `/api/users/me/avatar${q}`
}
