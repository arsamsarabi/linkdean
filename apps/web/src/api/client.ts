import type { z } from 'zod'
import { getToken } from '@/lib'
import { banNoticeSchema, type BanNotice } from '@/schemas'

export { getToken, setToken } from '@/lib'

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

export async function req<T>(path: string, init: RequestInit = {}, schema?: z.ZodType<T>): Promise<T> {
  const headers = new Headers(init.headers)
  if (!headers.has('Content-Type') && init.body && !(init.body instanceof FormData)) {
    headers.set('Content-Type', 'application/json')
  }
  const token = getToken()
  if (token) headers.set('Authorization', 'Bearer ' + token)
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
