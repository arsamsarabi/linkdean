import { z } from 'zod'

/** `file`: the address served a file (PDF, image, audio, video), kept as it was. */
export const archiveFormatSchema = z.enum(['html', 'pdf', 'file'])

export const archiveStatusSchema = z.enum(['queued', 'running', 'done', 'failed'])

/** One copy of a bookmark, stored or on its way. */
export const bookmarkArchiveSchema = z.object({
  format: archiveFormatSchema,
  status: archiveStatusSchema,
  bytes: z.number().int(),
  content_type: z.string().nullish(),
  captured_at: z.string().nullish(),
  /** Why the latest attempt failed. Set together with `available` when an older copy is kept. */
  error: z.string().nullish(),
  available: z.boolean(),
})

/** What a bookmark card knows about its copies. */
export const archiveSummarySchema = z.object({
  formats: z.array(archiveFormatSchema),
  captured_at: z.string().nullish(),
  pending: z.boolean(),
  failed: z.boolean(),
})

export const bulkArchiveResultSchema = z.object({
  ok: z.number().int(),
  failed: z.array(z.object({ id: z.string(), error: z.string() })),
})

export type ArchiveFormat = z.infer<typeof archiveFormatSchema>
export type BookmarkArchive = z.infer<typeof bookmarkArchiveSchema>
export type ArchiveSummary = z.infer<typeof archiveSummarySchema>
export type BulkArchiveResult = z.infer<typeof bulkArchiveResultSchema>
