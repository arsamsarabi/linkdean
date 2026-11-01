/** Where someone with no account picks up a share invite. */
export function shareClaimLink(token: string): string {
  return `${window.location.origin}/register?share=${encodeURIComponent(token)}`
}

type InviteOutcome = { status: string; invite?: { claim_token?: string } }

export function claimLinkFrom(outcome: InviteOutcome): string | null {
  const token = outcome.status === 'pending' ? outcome.invite?.claim_token : undefined
  return token ? shareClaimLink(token) : null
}
