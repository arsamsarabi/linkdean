import { z } from 'zod'

export const emailStatusSchema = z.object({
  enabled: z.boolean(),
  config: z
    .object({
      host: z.string(),
      port: z.number(),
      tls: z.enum(['starttls', 'tls', 'none']),
      from: z.string(),
    })
    .nullable(),
  stats: z.object({
    sent: z.number(),
    queued: z.number(),
    failed: z.number(),
    last_error: z.string().nullable(),
    last_error_at: z.string().nullable(),
  }),
})

export type EmailStatus = z.infer<typeof emailStatusSchema>

export const testEmailResultSchema = z.object({
  ok: z.boolean(),
  to: z.string(),
  error: z.string().optional(),
})

export type TestEmailResult = z.infer<typeof testEmailResultSchema>
