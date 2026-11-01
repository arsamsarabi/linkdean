import { useState, type ReactElement } from 'react'
import {
  ActionIcon,
  Alert,
  Button,
  Code,
  CopyButton,
  Divider,
  Drawer,
  Group,
  Select,
  Skeleton,
  Stack,
  Switch,
  Text,
  TextInput,
  Tooltip,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCheck, IconCopy, IconRefresh, IconTrash } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'
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

const accessOptions = [
  { value: 'read', label: 'Read-only' },
  { value: 'write', label: 'Read-write' },
]

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

  const activeShares = shares.data?.shares ?? []
  const pendingInvites = shares.data?.pending ?? []

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
      <Stack gap="md">
        <Text size="sm" c="dimmed">
          Invite by email. Existing users get access now; unknown emails stay pending until they
          register. Read-write can edit bookmarks and add subfolders (not vault, share, or delete).
        </Text>

        <Stack gap="sm">
          <TextInput
            label="Email"
            placeholder="user@example.com"
            value={email}
            onChange={(e) => setEmail(e.currentTarget.value)}
          />
          <Select
            label="Access"
            data={accessOptions}
            value={access}
            onChange={(v) => setAccess((v as ShareAccess) || 'read')}
            allowDeselect={false}
          />
          <Button
            onClick={() => void onInvite()}
            disabled={!email.trim()}
            loading={create.isPending}
          >
            Invite
          </Button>
        </Stack>

        {shares.isError ? (
          <QueryErrorAlert error={shares.error} onRetry={() => void shares.refetch()} />
        ) : null}

        {shares.isLoading ? (
          <Stack gap="xs">
            <Skeleton height={28} />
            <Skeleton height={28} width="80%" />
          </Stack>
        ) : (
          <Stack gap="xs">
            {activeShares.length === 0 && pendingInvites.length === 0 ? (
              <Text size="sm" c="dimmed">
                No invitees yet.
              </Text>
            ) : null}
            {activeShares.map((s) => (
              <Group key={s.id} justify="space-between" wrap="nowrap">
                <Stack gap={0} style={{ minWidth: 0, flex: 1 }}>
                  <Text size="sm" truncate>
                    {s.invitee_display_name || s.invitee_email}
                  </Text>
                  {s.invitee_display_name ? (
                    <Text size="xs" c="dimmed" truncate>
                      {s.invitee_email}
                    </Text>
                  ) : null}
                </Stack>
                <Select
                  size="xs"
                  data={accessOptions}
                  value={s.access}
                  allowDeselect={false}
                  onChange={(v) => {
                    if (!categoryId || !v) return
                    void update
                      .mutateAsync({
                        categoryId,
                        userId: s.invitee_user_id,
                        access: v as ShareAccess,
                      })
                      .catch((e) => notifications.show({ color: 'red', message: String(e) }))
                  }}
                  w={130}
                />
                <Tooltip label="Revoke">
                  <ActionIcon
                    size="sm"
                    variant="subtle"
                    color="red"
                    aria-label="Revoke share"
                    loading={remove.isPending}
                    onClick={() => {
                      if (!categoryId) return
                      void remove
                        .mutateAsync({ categoryId, userId: s.invitee_user_id })
                        .then(() => notifications.show({ message: 'Access revoked' }))
                        .catch((e) => notifications.show({ color: 'red', message: String(e) }))
                    }}
                  >
                    <IconTrash size={14} stroke={1.5} />
                  </ActionIcon>
                </Tooltip>
              </Group>
            ))}
            {pendingInvites.map((p) => (
              <Group key={p.id} justify="space-between" wrap="nowrap">
                <Stack gap={0} style={{ minWidth: 0, flex: 1 }}>
                  <Text size="sm" truncate>
                    {p.invitee_email}
                  </Text>
                  <Text size="xs" c="dimmed">
                    Pending — claims on register
                  </Text>
                </Stack>
                <Select
                  size="xs"
                  data={accessOptions}
                  value={p.access}
                  allowDeselect={false}
                  onChange={(v) => {
                    if (!categoryId || !v) return
                    void updatePending
                      .mutateAsync({
                        categoryId,
                        pendingId: p.id,
                        access: v as ShareAccess,
                      })
                      .catch((e) => notifications.show({ color: 'red', message: String(e) }))
                  }}
                  w={130}
                />
                <Tooltip label="Cancel pending invite">
                  <ActionIcon
                    size="sm"
                    variant="subtle"
                    color="red"
                    aria-label="Cancel pending invite"
                    loading={removePending.isPending}
                    onClick={() => {
                      if (!categoryId) return
                      void removePending
                        .mutateAsync({ categoryId, pendingId: p.id })
                        .then(() => notifications.show({ message: 'Pending invite cancelled' }))
                        .catch((e) => notifications.show({ color: 'red', message: String(e) }))
                    }}
                  >
                    <IconTrash size={14} stroke={1.5} />
                  </ActionIcon>
                </Tooltip>
              </Group>
            ))}
          </Stack>
        )}

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
      </Stack>
    </Drawer>
  )
}
