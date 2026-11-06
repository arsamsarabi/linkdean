import { z } from 'zod'

export const abuseTargetTypeSchema = z.enum(['public_list', 'profile', 'comment'])
export const abuseReasonSchema = z.enum(['spam', 'harassment', 'illegal', 'impersonation', 'other'])
export const abuseActionSchema = z.enum(['dismiss', 'hide', 'warn', 'ban'])

export const abuseReportSchema = z.object({
  id: z.string(),
  reporter_user_id: z.string(),
  reporter_email: z.string().nullish(),
  target_type: abuseTargetTypeSchema,
  target_id: z.string(),
  target_user_id: z.string().nullish(),
  target_label: z.string(),
  reason: z.string(),
  details: z.string().nullish(),
  status: z.enum(['open', 'resolved']),
  resolution_action: z.string().nullish(),
  resolution_note: z.string().nullish(),
  created_at: z.string(),
  resolved_at: z.string().nullish(),
})

export const abuseReportsEnvelopeSchema = z.object({ reports: z.array(abuseReportSchema) })
export const abuseReportEnvelopeSchema = z.object({ report: abuseReportSchema })
export const reportFiledSchema = z.object({ ok: z.boolean(), id: z.string() })
export const reportConfigSchema = z.object({ abuse_contact_email: z.string().nullish() })

export type AbuseTargetType = z.infer<typeof abuseTargetTypeSchema>
export type AbuseReason = z.infer<typeof abuseReasonSchema>
export type AbuseAction = z.infer<typeof abuseActionSchema>
export type AbuseReport = z.infer<typeof abuseReportSchema>
export type ReportConfig = z.infer<typeof reportConfigSchema>
export type NewAbuseReport = {
  target_type: AbuseTargetType
  target_id: string
  reason: AbuseReason
  details?: string
}
export type ResolveAbuseReport = { action: AbuseAction; note?: string }
