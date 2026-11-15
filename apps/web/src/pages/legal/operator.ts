/**
 * Who runs this instance, as named in the Terms of Service and the Privacy Policy.
 *
 * Known issue: these are linkdean.app's details, so every self-hosted instance also names
 * Arsam Sarabi as its operator and data controller. Suggested fix: move them into the admin
 * platform settings next to `abuse_contact_email`, serve them on a public endpoint, and fall
 * back to "the operator of this instance" when a value is not set.
 */
export const OPERATOR = {
  legalName: 'Arsam Sarabi',
  tradingName: 'Linkdean',
  contactEmail: 'contact@linkdean.app',
  privacyEmail: 'privacy@linkdean.app',
  abuseEmail: 'abuse@linkdean.app',
  governingLaw: 'England and Wales',
} as const
