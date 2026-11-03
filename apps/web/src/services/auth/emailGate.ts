import type { MeResponse } from '@/schemas'

/** True when the hard gate holds this account: the gate is on, it is unverified and not an admin. */
export function isGated(me: MeResponse): boolean {
  return me.require_verified_email && !me.user.email_verified && !me.user.is_admin
}
