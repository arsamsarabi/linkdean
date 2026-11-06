import type { AbuseReason } from '@/schemas'

/** The reasons the report form offers; the server accepts exactly these. */
export const REPORT_REASONS: readonly { value: AbuseReason; label: string }[] = [
  { value: 'spam', label: 'Spam or scam' },
  { value: 'harassment', label: 'Harassment or hate' },
  { value: 'illegal', label: 'Illegal or harmful content' },
  { value: 'impersonation', label: 'Impersonation' },
  { value: 'other', label: 'Something else' },
] as const

export const REPORT_DETAILS_MAX = 2000
