import { z } from 'zod'
import { sessionItem, type Session } from './session'

/** The session ended on the server (key revoked, account banned): sign in again. */
export class SignedOutError extends Error {
  constructor() {
    super('You were signed out of Linkdean. Sign in again.')
  }
}

export const categorySchema = z.object({
  id: z.string(),
  parent_id: z.string().nullish(),
  name: z.string(),
  is_vault: z.boolean(),
  access: z.string().default('owner'),
})
export type Category = z.infer<typeof categorySchema>

export const bookmarkSchema = z.object({
  id: z.string(),
  url: z.string(),
  title: z.string().nullish(),
})
export type Bookmark = z.infer<typeof bookmarkSchema>

function errorMessage(body: unknown, status: number): string {
  if (body && typeof body === 'object' && 'error' in body && typeof body.error === 'string') return body.error
  return `Linkdean answered ${status}.`
}

/** Calls the signed-in instance with the session's key. A 401 ends the session. */
export async function call<T>(
  session: Session,
  path: string,
  schema: z.ZodType<T>,
  init: RequestInit = {}
): Promise<T> {
  const res = await fetch(`${session.instance}/api${path}`, {
    ...init,
    headers: {
      Authorization: `Bearer ${session.token}`,
      'Content-Type': 'application/json',
      ...init.headers,
    },
  })
  const body: unknown = res.headers.get('content-type')?.includes('application/json') ? await res.json() : null
  if (res.status === 401) {
    await sessionItem.setValue(null)
    throw new SignedOutError()
  }
  if (!res.ok) throw new Error(errorMessage(body, res.status))
  return schema.parse(body)
}

export function lookup(session: Session, url: string): Promise<Bookmark | null> {
  return call(
    session,
    `/bookmarks/lookup?url=${encodeURIComponent(url)}`,
    z.object({ bookmark: bookmarkSchema.nullable() })
  ).then((r) => r.bookmark)
}

export function categories(session: Session): Promise<Category[]> {
  return call(session, '/categories?include_hidden=false&include_vault=false', z.array(categorySchema))
}

export function tags(session: Session): Promise<string[]> {
  return call(session, '/tags', z.array(z.object({ name: z.string() }))).then((t) => t.map((x) => x.name))
}

export type NewBookmark = {
  url: string
  title?: string
  description?: string
  category_ids: string[]
  tags: string[]
}

export function createBookmark(session: Session, body: NewBookmark): Promise<Bookmark> {
  return call(session, '/bookmarks', bookmarkSchema, { method: 'POST', body: JSON.stringify(body) })
}

/** Revokes the extension's own key, so signing out leaves nothing usable behind. */
export function revokeOwnKey(session: Session): Promise<unknown> {
  return call(session, `/api-keys/${session.keyId}`, z.unknown(), { method: 'DELETE' })
}

/** The folders a new bookmark can go into, with their full path, sorted by it. */
export function folderOptions(cats: Category[]): { value: string; label: string }[] {
  const byId = new Map(cats.map((c) => [c.id, c]))
  const path = (c: Category): string => {
    const names: string[] = []
    let cur: Category | undefined = c
    const seen = new Set<string>()
    while (cur && !seen.has(cur.id)) {
      seen.add(cur.id)
      names.unshift(cur.name)
      cur = cur.parent_id ? byId.get(cur.parent_id) : undefined
    }
    return names.join(' / ')
  }
  return cats
    .filter((c) => !c.is_vault && (c.access === 'owner' || c.access === 'write'))
    .map((c) => ({ value: c.id, label: path(c) }))
    .sort((a, b) => a.label.localeCompare(b.label))
}

/** True when the page can be bookmarked: Linkdean saves only http(s) addresses. */
export function savableUrl(url: string | undefined): url is string {
  return !!url && /^https?:\/\//i.test(url)
}
