import type { BanDurationBody } from '@/schemas'

export const MAX_BAN_DAYS = 3650
export const MAX_BAN_REASON = 500

export type BanDurationForm = {
  mode: 'permanent' | 'days' | 'date'
  days: number | string
  date: string
}

export const emptyBanDuration: BanDurationForm = { mode: 'permanent', days: 7, date: '' }

/** Request fields for a duration, or null when the form is not valid yet. */
export function banDurationBody(form: BanDurationForm, now: Date = new Date()): BanDurationBody | null {
  if (form.mode === 'permanent') return { permanent: true }
  if (form.mode === 'days') {
    const days = Number(form.days)
    return Number.isInteger(days) && days >= 1 && days <= MAX_BAN_DAYS ? { days } : null
  }
  if (!form.date) return null
  const end = new Date(`${form.date}T23:59:59`)
  return end > now ? { ends_at: end.toISOString() } : null
}

export function validBanReason(reason: string): boolean {
  const length = reason.trim().length
  return length >= 1 && length <= MAX_BAN_REASON
}

export function banEndLabel(endsAt: string | null | undefined): string {
  return endsAt ? `until ${new Date(endsAt).toLocaleDateString()}` : 'permanently'
}
