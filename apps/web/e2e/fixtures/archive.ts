import { execFileSync } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { mkdirSync, writeFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import type { APIRequestContext } from '@playwright/test'
import { CLIENT_HEADER } from './api'

const here = dirname(fileURLToPath(import.meta.url))
/** Repo-root `data/archives` (API default when cwd is the monorepo root). */
const defaultArchiveDir = resolve(here, '../../../../data/archives')

function archiveRoot(): string {
  return process.env.E2E_ARCHIVE_DIR ?? process.env.ARCHIVE_DIR ?? defaultArchiveDir
}

/** Smoke Meili indexes created before page-text may omit it from searchable attributes. */
async function ensureMeiliSearchesPageText(): Promise<void> {
  const base = (process.env.MEILI_URL ?? 'http://127.0.0.1:57700').replace(/\/$/, '')
  const key = process.env.MEILI_MASTER_KEY ?? 'smoke-master-key-0001'
  const res = await fetch(`${base}/indexes/bookmarks/settings/searchable-attributes`, {
    method: 'PUT',
    headers: { Authorization: `Bearer ${key}`, 'Content-Type': 'application/json' },
    body: JSON.stringify(['title', 'url', 'description', 'notes', 'tags', 'page_text']),
  })
  if (!res.ok) {
    throw new Error(`Meili searchable-attributes update failed: ${res.status} ${await res.text()}`)
  }
  const task = (await res.json()) as { taskUid?: number }
  if (task.taskUid == null) return
  for (let i = 0; i < 40; i++) {
    const t = await fetch(`${base}/tasks/${task.taskUid}`, { headers: { Authorization: `Bearer ${key}` } })
    const body = (await t.json()) as { status?: string }
    if (body.status === 'succeeded') return
    if (body.status === 'failed') throw new Error(`Meili task ${task.taskUid} failed`)
    await new Promise((r) => setTimeout(r, 100))
  }
}

/** Runs SQL against the throwaway smoke DB (psql via DATABASE_URL, else `docker exec pg-check`). */
function runSql(sql: string): void {
  const url = process.env.E2E_DATABASE_URL ?? process.env.DATABASE_URL
  if (url) {
    try {
      execFileSync('psql', [url, '-v', 'ON_ERROR_STOP=1', '-c', sql], { encoding: 'utf8', stdio: 'pipe' })
      return
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code !== 'ENOENT') throw e
    }
  }
  execFileSync(
    'docker',
    ['exec', 'pg-check', 'psql', '-U', 'linkdean', '-d', 'smoke', '-v', 'ON_ERROR_STOP=1', '-c', sql],
    { encoding: 'utf8', stdio: 'pipe' }
  )
}

/**
 * Writes a finished HTML archive row + file so UI tests do not need Chrome.
 * When `pageText` is set, patches the bookmark so Meili picks up body search.
 */
export async function seedFinishedHtmlArchive(
  request: APIRequestContext,
  bookmarkId: string,
  opts: { pageText?: string; htmlBody?: string } = {}
): Promise<{ fileName: string; capturedAt: string }> {
  const fileName = `html-e2e-${randomUUID().slice(0, 8)}.html`
  const phrase = opts.pageText ?? ''
  const html =
    opts.htmlBody ??
    `<!DOCTYPE html><html><body><article><p>${phrase || 'e2e archived page'}</p></article></body></html>`
  const dir = join(archiveRoot(), bookmarkId)
  mkdirSync(dir, { recursive: true })
  writeFileSync(join(dir, fileName), html, 'utf8')
  const bytes = Buffer.byteLength(html, 'utf8')
  const capturedAt = new Date().toISOString()
  // Smoke DBs may still be on migration 059; page text ships in 060.
  runSql('ALTER TABLE bookmark_archives ADD COLUMN IF NOT EXISTS page_text TEXT')
  const pageTextSql = phrase ? `$pt$${phrase}$pt$` : 'NULL'
  // UUID and file name are generated here; page text is dollar-quoted.
  runSql(`
    INSERT INTO bookmark_archives (
      bookmark_id, format, status, file_name, bytes, content_type,
      final_url, captured_at, queued_at, started_at, page_text
    ) VALUES (
      '${bookmarkId}'::uuid, 'html', 'done', '${fileName}', ${bytes}, 'text/html',
      'https://example.com/e2e-archive', '${capturedAt}'::timestamptz, now(), now(), ${pageTextSql}
    )
    ON CONFLICT (bookmark_id, format) DO UPDATE SET
      status = EXCLUDED.status,
      file_name = EXCLUDED.file_name,
      bytes = EXCLUDED.bytes,
      content_type = EXCLUDED.content_type,
      final_url = EXCLUDED.final_url,
      captured_at = EXCLUDED.captured_at,
      page_text = EXCLUDED.page_text,
      error = NULL,
      key_vault_id = NULL,
      nonce = NULL
  `)
  if (phrase) await ensureMeiliSearchesPageText()
  // Metadata patch re-runs search_index::upsert, which loads page_text for Meili.
  const patch = await request.patch(`/api/bookmarks/${bookmarkId}`, {
    headers: CLIENT_HEADER,
    data: { notes: `e2e archive seed ${randomUUID().slice(0, 8)}` },
  })
  if (!patch.ok()) {
    throw new Error(`seed archive Meili refresh failed: ${patch.status()} ${await patch.text()}`)
  }
  return { fileName, capturedAt }
}
