/** What happened to the invite email after creating a share. */
export type InviteEmailStatus = 'sent' | 'limited' | null

type InviteEmailOutcome = { email_sent?: boolean; email_limited?: boolean }

export function inviteEmailStatus(outcome: InviteEmailOutcome): InviteEmailStatus {
  if (outcome.email_limited) return 'limited'
  if (outcome.email_sent) return 'sent'
  return null
}

export const INVITE_EMAIL_SENT = 'We emailed them an invite.'
export const INVITE_EMAIL_LIMITED =
  'The invite was saved, but no email was sent: too many invites in the last hour. Copy the link instead.'

/** The toast after creating a share; the panel shows the email outcome itself. */
export function inviteToast(claimLink: string | null, email: InviteEmailStatus): string {
  if (email) return 'Invite created.'
  return claimLink ? 'Invite created. Send them the link.' : 'Invite sent'
}
