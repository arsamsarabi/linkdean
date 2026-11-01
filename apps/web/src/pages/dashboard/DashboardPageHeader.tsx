import { type ReactElement } from 'react'
import { ActionIcon, Badge, Button, Group, Menu, Select, Stack, Text, TextInput, Title } from '@mantine/core'
import {
  IconDeviceFloppy,
  IconDotsVertical,
  IconMessageCircle,
  IconPencil,
  IconPin,
  IconPlus,
  IconShare,
  IconTrash,
  IconX,
} from '@tabler/icons-react'
import { ShareBadge } from '@/components'
import type { Dashboard } from '@/schemas'
import type { DashboardDraft } from '@/services'

const iconSm = { size: 16, stroke: 1.5 } as const

type Props = {
  view: Dashboard
  draft: DashboardDraft | null
  activeId: string
  switcherData: { value: string; label: string }[]
  isOwner: boolean
  isPinned: boolean
  canEditLayout: boolean
  socialEnabled: boolean
  inviteeAccess: 'read' | 'write' | null
  createPending: boolean
  pinPending: boolean
  savePending: boolean
  onDraftTitleChange: (title: string) => void
  onSelectDashboard: (id: string | null) => void
  onCreate: () => void
  onOpenSocial: () => void
  onOpenShare: () => void
  onPin: () => void
  onOpenDelete: () => void
  onCancelEdit: () => void
  onSave: () => void
  onStartEdit: () => void
}

export function DashboardPageHeader({
  view,
  draft,
  activeId,
  switcherData,
  isOwner,
  isPinned,
  canEditLayout,
  socialEnabled,
  inviteeAccess,
  createPending,
  pinPending,
  savePending,
  onDraftTitleChange,
  onSelectDashboard,
  onCreate,
  onOpenSocial,
  onOpenShare,
  onPin,
  onOpenDelete,
  onCancelEdit,
  onSave,
  onStartEdit,
}: Props): ReactElement {
  return (
    <Stack gap="xs">
      <Group justify="space-between" align="center" wrap="wrap" gap="md">
        {draft ? (
          <TextInput
            aria-label="Dashboard title"
            value={draft.title}
            onChange={(e) => onDraftTitleChange(e.currentTarget.value)}
            style={{ flex: '1 1 220px', minWidth: 220, maxWidth: 420 }}
            styles={{ input: { fontSize: '1.5rem', fontWeight: 700, height: 42 } }}
          />
        ) : (
          <Group gap="sm" align="center" wrap="wrap" style={{ flex: '1 1 220px', minWidth: 220 }}>
            <Title order={2}>{view.title}</Title>
            {inviteeAccess ? <ShareBadge access={inviteeAccess} ownerLabel={view.owner_display_name} /> : null}
            {isOwner && view.shared_out ? <ShareBadge outgoing sharedWith={view.shared_with} /> : null}
            {view.deleted_at ? (
              <Badge size="sm" variant="outline" color="red" tt="none">
                Pending delete
              </Badge>
            ) : null}
          </Group>
        )}
        <Group gap="sm" wrap="nowrap" align="center">
          {!draft ? (
            <Select
              aria-label="Switch dashboard"
              placeholder="Dashboards"
              data={switcherData}
              value={activeId}
              onChange={onSelectDashboard}
              allowDeselect={false}
              maw={220}
            />
          ) : null}
          {!draft ? (
            <Menu shadow="md" width={200} position="bottom-end" withinPortal>
              <Menu.Target>
                <ActionIcon variant="default" size={36} aria-label="Dashboard actions">
                  <IconDotsVertical size={16} stroke={1.5} />
                </ActionIcon>
              </Menu.Target>
              <Menu.Dropdown>
                <Menu.Item
                  leftSection={<IconPlus size={14} stroke={1.5} />}
                  onClick={onCreate}
                  disabled={createPending}
                >
                  New dashboard
                </Menu.Item>
                {socialEnabled ? (
                  <Menu.Item leftSection={<IconMessageCircle size={14} stroke={1.5} />} onClick={onOpenSocial}>
                    Likes & comments
                  </Menu.Item>
                ) : null}
                {isOwner ? (
                  <Menu.Item leftSection={<IconShare size={14} stroke={1.5} />} onClick={onOpenShare}>
                    Share…
                  </Menu.Item>
                ) : null}
                <Menu.Item
                  leftSection={<IconPin size={14} stroke={1.5} />}
                  onClick={onPin}
                  disabled={isPinned || pinPending}
                >
                  {isPinned ? 'Already default' : 'Set as default'}
                </Menu.Item>
                {isOwner ? (
                  <>
                    <Menu.Divider />
                    <Menu.Item color="red" leftSection={<IconTrash size={14} stroke={1.5} />} onClick={onOpenDelete}>
                      Delete dashboard
                    </Menu.Item>
                  </>
                ) : null}
              </Menu.Dropdown>
            </Menu>
          ) : null}
          {draft ? (
            <>
              <Button type="button" variant="default" leftSection={<IconX {...iconSm} />} onClick={onCancelEdit}>
                Cancel
              </Button>
              <Button
                type="button"
                leftSection={<IconDeviceFloppy {...iconSm} />}
                onClick={onSave}
                loading={savePending}
              >
                Save
              </Button>
            </>
          ) : canEditLayout ? (
            <Button type="button" leftSection={<IconPencil {...iconSm} />} variant="light" onClick={onStartEdit}>
              Edit layout
            </Button>
          ) : null}
        </Group>
      </Group>
      <Text size="sm" c="dimmed">
        {draft
          ? 'Editing layout — pick bookmarks, then Save. Manage bookmarks in the library.'
          : 'Glance at pinned links. Switch boards or set a default pin.'}
      </Text>
    </Stack>
  )
}
