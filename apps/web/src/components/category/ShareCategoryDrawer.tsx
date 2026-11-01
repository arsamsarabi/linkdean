import { type ReactElement } from 'react'
import { Divider, Drawer } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { ShareInvitePanel } from '@/components/share-invite'
import { useShareCategoryDrawer } from '@/hooks/categories'
import { CategoryPublicLinkSection } from './CategoryPublicLinkSection'

type Props = {
  categoryId: string | null
  categoryName: string
  /** Vault lists cannot publish a public URL. */
  isVault?: boolean
  opened: boolean
  onClose: () => void
}

export function ShareCategoryDrawer({
  categoryId,
  categoryName,
  isVault = false,
  opened,
  onClose,
}: Props): ReactElement {
  const s = useShareCategoryDrawer({ categoryId, opened, isVault })

  return (
    <Drawer opened={opened} onClose={onClose} title={`Share “${categoryName}”`} position="right" size="md" padding="lg">
      <ShareInvitePanel
        description="Invite by email. They get access once they accept."
        email={s.email}
        access={s.access}
        onEmailChange={s.setEmail}
        onAccessChange={s.setAccess}
        onInvite={() => void s.onInvite()}
        invitePending={s.create.isPending}
        sharesError={s.shares.isError ? s.shares.error : null}
        onRetryShares={() => void s.shares.refetch()}
        sharesLoading={s.shares.isLoading}
        activeShares={s.shares.data?.shares ?? []}
        pendingInvites={s.shares.data?.pending ?? []}
        removeBusy={s.remove.isPending || s.removePending.isPending}
        onUpdateAccess={(userId, next) => {
          if (!categoryId) return
          void s.update
            .mutateAsync({ categoryId, userId, access: next })
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onRevoke={(userId) => {
          if (!categoryId) return
          void s.remove
            .mutateAsync({ categoryId, userId })
            .then(() => notifications.show({ message: 'Access revoked' }))
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onUpdatePendingAccess={(pendingId, next) => {
          if (!categoryId) return
          void s.updatePending
            .mutateAsync({ categoryId, pendingId, access: next })
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onCancelPending={(pendingId) => {
          if (!categoryId) return
          void s.removePending
            .mutateAsync({ categoryId, pendingId })
            .then(() => notifications.show({ message: 'Pending invite cancelled' }))
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
      >
        <Divider label="Public link" labelPosition="left" />
        <CategoryPublicLinkSection
          isVault={isVault}
          loading={s.publicLink.isLoading}
          error={s.publicLink.isError ? s.publicLink.error : null}
          onRetry={() => void s.publicLink.refetch()}
          enabled={!!s.publicLink.data?.enabled}
          publicUrl={s.publicUrl}
          publicPath={s.publicPath ?? undefined}
          busy={s.publicBusy}
          rotatePending={s.rotatePublic.isPending}
          onToggle={(next) => void s.onTogglePublic(next)}
          onRotate={() => void s.onRotatePublic()}
        />
      </ShareInvitePanel>
    </Drawer>
  )
}
