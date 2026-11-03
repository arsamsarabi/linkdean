import { useState, type ReactElement } from 'react'
import { Drawer } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { ShareInvitePanel } from '@/components/share-invite'
import {
  useCreateDashboardShare,
  useDashboardShares,
  useDeleteDashboardPendingShare,
  useDeleteDashboardShare,
  useUpdateDashboardPendingShare,
  useUpdateDashboardShare,
} from '@/hooks'
import type { ShareAccess } from '@/schemas'
import { claimLinkFrom, inviteEmailStatus, inviteToast, type InviteEmailStatus } from '@/services'

type Props = {
  dashboardId: string | null
  dashboardTitle: string
  opened: boolean
  onClose: () => void
}

export function ShareDashboardDrawer({ dashboardId, dashboardTitle, opened, onClose }: Props): ReactElement {
  const [email, setEmail] = useState('')
  const [access, setAccess] = useState<ShareAccess>('read')

  const shares = useDashboardShares(opened ? dashboardId : null)
  const create = useCreateDashboardShare()
  const update = useUpdateDashboardShare()
  const remove = useDeleteDashboardShare()
  const updatePending = useUpdateDashboardPendingShare()
  const removePending = useDeleteDashboardPendingShare()

  const [claimLink, setClaimLink] = useState<string | null>(null)
  const [inviteEmail, setInviteEmail] = useState<InviteEmailStatus>(null)

  async function onInvite(): Promise<void> {
    if (!dashboardId || !email.trim()) return
    try {
      const outcome = await create.mutateAsync({
        dashboardId,
        email: email.trim(),
        access,
      })
      setEmail('')
      const link = claimLinkFrom(outcome)
      const emailed = inviteEmailStatus(outcome)
      setClaimLink(link)
      setInviteEmail(emailed)
      notifications.show({ message: inviteToast(link, emailed) })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={`Share “${dashboardTitle}”`}
      position="right"
      size="md"
      padding="lg"
    >
      <ShareInvitePanel
        description="Invite by email. They get access once they accept."
        email={email}
        access={access}
        onEmailChange={setEmail}
        onAccessChange={setAccess}
        onInvite={() => void onInvite()}
        claimLink={claimLink}
        inviteEmail={inviteEmail}
        invitePending={create.isPending}
        sharesError={shares.isError ? shares.error : null}
        onRetryShares={() => void shares.refetch()}
        sharesLoading={shares.isLoading}
        activeShares={shares.data?.shares ?? []}
        pendingInvites={shares.data?.pending ?? []}
        removeBusy={remove.isPending || removePending.isPending}
        onUpdateAccess={(userId, next) => {
          if (!dashboardId) return
          void update
            .mutateAsync({ dashboardId, userId, access: next })
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onRevoke={(userId) => {
          if (!dashboardId) return
          void remove
            .mutateAsync({ dashboardId, userId })
            .then(() => notifications.show({ message: 'Access revoked' }))
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onUpdatePendingAccess={(pendingId, next) => {
          if (!dashboardId) return
          void updatePending
            .mutateAsync({ dashboardId, pendingId, access: next })
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onCancelPending={(pendingId) => {
          if (!dashboardId) return
          void removePending
            .mutateAsync({ dashboardId, pendingId })
            .then(() => notifications.show({ message: 'Pending invite cancelled' }))
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
      />
    </Drawer>
  )
}
