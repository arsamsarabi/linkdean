import { useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Alert,
  Code,
  CopyButton,
  Divider,
  Drawer,
  Group,
  Skeleton,
  Stack,
  Switch,
  Text,
  TextInput,
  Tooltip,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCheck, IconCopy, IconRefresh } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { ShareInvitePanel } from '@/components/share-invite'
import {
  useCategoryPublicLink,
  useCategoryShares,
  useCreateCategoryShare,
  useDeleteCategoryPendingShare,
  useDeleteCategoryShare,
  useDisableCategoryPublicLink,
  useEnableCategoryPublicLink,
  useRotateCategoryPublicLink,
  useUpdateCategoryPendingShare,
  useUpdateCategoryShare,
} from '@/hooks'
import type { ShareAccess } from '@/schemas'

type Props = {
  categoryId: string | null
  categoryName: string
  /** Vault lists cannot publish a public URL. */
  isVault?: boolean
  opened: boolean
  onClose: () => void
}

function absolutePublicUrl(path: string): string {
  if (typeof window === 'undefined') return path
  return `${window.location.origin}${path}`
}

export function ShareCategoryDrawer({
  categoryId,
  categoryName,
  isVault = false,
  opened,
  onClose,
}: Props): ReactElement {
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
        message:
          outcome.status === 'pending'
            ? 'No account yet — access granted when they register'
            : 'Invite added',
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
  const publicBusy =
    enablePublic.isPending || rotatePublic.isPending || disablePublic.isPending

  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      title={`Share “${categoryName}”`}
      position="right"
      size="md"
      padding="lg"
    >
      <ShareInvitePanel
        description="Invite by email. Existing users get access now; unknown emails stay pending until they register. Read-write can edit bookmarks and add subfolders (not vault, share, or delete)."
        email={email}
        access={access}
        onEmailChange={setEmail}
        onAccessChange={setAccess}
        onInvite={() => void onInvite()}
        invitePending={create.isPending}
        sharesError={shares.isError ? shares.error : null}
        onRetryShares={() => void shares.refetch()}
        sharesLoading={shares.isLoading}
        activeShares={shares.data?.shares ?? []}
        pendingInvites={shares.data?.pending ?? []}
        removeBusy={remove.isPending || removePending.isPending}
        onUpdateAccess={(userId, next) => {
          if (!categoryId) return
          void update
            .mutateAsync({ categoryId, userId, access: next })
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onRevoke={(userId) => {
          if (!categoryId) return
          void remove
            .mutateAsync({ categoryId, userId })
            .then(() => notifications.show({ message: 'Access revoked' }))
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onUpdatePendingAccess={(pendingId, next) => {
          if (!categoryId) return
          void updatePending
            .mutateAsync({ categoryId, pendingId, access: next })
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
        onCancelPending={(pendingId) => {
          if (!categoryId) return
          void removePending
            .mutateAsync({ categoryId, pendingId })
            .then(() => notifications.show({ message: 'Pending invite cancelled' }))
            .catch((e) => notifications.show({ color: 'red', message: String(e) }))
        }}
      >
        <Divider label="Public link" labelPosition="left" />

        {isVault ? (
          <Alert color="gray" variant="light">
            Vault lists cannot be published. Public links never include vault bookmarks.
          </Alert>
        ) : (
          <Stack gap="sm">
            <Text size="sm" c="dimmed">
              Anyone with the link can view this list (read-only). Unlisted — not searchable. Notes
              stay private.
            </Text>

            {publicLink.isError ? (
              <QueryErrorAlert
                error={publicLink.error}
                onRetry={() => void publicLink.refetch()}
              />
            ) : null}

            {publicLink.isLoading ? (
              <Skeleton height={36} />
            ) : (
              <Switch
                label="Enable public URL"
                checked={!!publicLink.data?.enabled}
                disabled={publicBusy}
                onChange={(e) => void onTogglePublic(e.currentTarget.checked)}
              />
            )}

            {publicLink.data?.enabled && publicUrl ? (
              <Stack gap="xs">
                <Group gap="xs" wrap="nowrap" align="flex-end">
                  <TextInput
                    label="Link"
                    value={publicUrl}
                    readOnly
                    style={{ flex: 1 }}
                    styles={{ input: { fontFamily: 'var(--mantine-font-family-monospace)' } }}
                  />
                  <CopyButton value={publicUrl} timeout={1500}>
                    {({ copied, copy }) => (
                      <Tooltip label={copied ? 'Copied' : 'Copy'}>
                        <ActionIcon
                          variant="light"
                          color={copied ? 'teal' : 'violet'}
                          aria-label="Copy public link"
                          onClick={copy}
                          mb={2}
                        >
                          {copied ? (
                            <IconCheck size={16} stroke={1.5} />
                          ) : (
                            <IconCopy size={16} stroke={1.5} />
                          )}
                        </ActionIcon>
                      </Tooltip>
                    )}
                  </CopyButton>
                  <Tooltip label="Rotate link">
                    <ActionIcon
                      variant="subtle"
                      aria-label="Rotate public link"
                      loading={rotatePublic.isPending}
                      onClick={() => void onRotatePublic()}
                      mb={2}
                    >
                      <IconRefresh size={16} stroke={1.5} />
                    </ActionIcon>
                  </Tooltip>
                </Group>
                <Text size="xs" c="dimmed">
                  Path <Code>{publicPath}</Code>
                </Text>
              </Stack>
            ) : null}
          </Stack>
        )}
      </ShareInvitePanel>
    </Drawer>
  )
}
