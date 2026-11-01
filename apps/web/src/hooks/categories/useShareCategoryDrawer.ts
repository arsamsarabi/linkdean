import { useState } from 'react'
import { notifications } from '@mantine/notifications'
import type { ShareAccess } from '@/schemas'
import {
  useCategoryPublicLink,
  useDisableCategoryPublicLink,
  useEnableCategoryPublicLink,
  useRotateCategoryPublicLink,
} from './useCategoryPublicLink'
import {
  useCategoryShares,
  useCreateCategoryShare,
  useDeleteCategoryPendingShare,
  useDeleteCategoryShare,
  useUpdateCategoryPendingShare,
  useUpdateCategoryShare,
} from './useCategoryShares'

function absolutePublicUrl(path: string): string {
  if (typeof window === 'undefined') return path
  return `${window.location.origin}${path}`
}

type Input = {
  categoryId: string | null
  opened: boolean
  isVault: boolean
}

/** Invite + public-link orchestration for ShareCategoryDrawer. */
export function useShareCategoryDrawer({ categoryId, opened, isVault }: Input) {
  const [email, setEmail] = useState('')
  const [access, setAccess] = useState<ShareAccess>('read')

  const shares = useCategoryShares(opened ? categoryId : null)
  const create = useCreateCategoryShare()
  const update = useUpdateCategoryShare()
  const remove = useDeleteCategoryShare()
  const updatePending = useUpdateCategoryPendingShare()
  const removePending = useDeleteCategoryPendingShare()

  const publicLink = useCategoryPublicLink(opened ? categoryId : null, !isVault)
  const enablePublic = useEnableCategoryPublicLink()
  const rotatePublic = useRotateCategoryPublicLink()
  const disablePublic = useDisableCategoryPublicLink()

  async function onInvite(): Promise<void> {
    if (!categoryId || !email.trim()) return
    try {
      const outcome = await create.mutateAsync({
        categoryId,
        email: email.trim(),
        access,
      })
      setEmail('')
      notifications.show({
        message: outcome.status === 'pending' ? 'No account yet — access granted when they register' : 'Invite added',
      })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onTogglePublic(next: boolean): Promise<void> {
    if (!categoryId) return
    try {
      if (next) {
        await enablePublic.mutateAsync(categoryId)
        notifications.show({ message: 'Public link enabled' })
      } else {
        await disablePublic.mutateAsync(categoryId)
        notifications.show({ message: 'Public link revoked' })
      }
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onRotatePublic(): Promise<void> {
    if (!categoryId) return
    try {
      await rotatePublic.mutateAsync(categoryId)
      notifications.show({ message: 'Public link rotated — old URL no longer works' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  const publicPath = publicLink.data?.path
  const publicUrl = publicPath ? absolutePublicUrl(publicPath) : ''
  const publicBusy = enablePublic.isPending || rotatePublic.isPending || disablePublic.isPending

  return {
    email,
    setEmail,
    access,
    setAccess,
    shares,
    create,
    update,
    remove,
    updatePending,
    removePending,
    publicLink,
    rotatePublic,
    publicPath,
    publicUrl,
    publicBusy,
    onInvite,
    onTogglePublic,
    onRotatePublic,
  }
}
