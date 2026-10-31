export type InviteeShareAccess = 'read' | 'write'

/** Access label when viewer is an invitee (not the owner). Null = own / unknown. */
export function inviteeShareAccess(
  ownerUserId: string | null | undefined,
  canWrite: boolean | undefined,
  viewerUserId: string | null | undefined
): InviteeShareAccess | null {
  if (!ownerUserId || !viewerUserId) return null
  if (ownerUserId === viewerUserId) return null
  return canWrite ? 'write' : 'read'
}
