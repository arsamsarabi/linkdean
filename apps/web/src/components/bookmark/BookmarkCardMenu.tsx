import { type ReactElement } from 'react'
import { ActionIcon, CopyButton, Menu, Tooltip } from '@mantine/core'
import {
  IconBookmark,
  IconBookmarkFilled,
  IconCheck,
  IconCopy,
  IconDotsVertical,
  IconHeart,
  IconMessageCircle,
  IconPencil,
  IconPin,
  IconPinFilled,
  IconStar,
  IconStarFilled,
  IconTrash,
} from '@tabler/icons-react'

type FlagPatch = Partial<{ is_favorite: boolean; is_unread: boolean; is_pinned: boolean }>

type Props = {
  url: string
  canWrite: boolean
  socialEnabled: boolean
  isFavorite?: boolean
  isUnread?: boolean
  isPinned?: boolean
  onFlags?: (patch: FlagPatch) => void
  onOpenSocial: () => void
  onEdit: () => void
  onDelete: () => void
}

export function BookmarkCardMenu({
  url,
  canWrite,
  socialEnabled,
  isFavorite,
  isUnread,
  isPinned,
  onFlags,
  onOpenSocial,
  onEdit,
  onDelete,
}: Props): ReactElement {
  return (
    <Menu shadow="md" width={200} position="bottom-end" withinPortal>
      <Menu.Target>
        <Tooltip label="Actions">
          <ActionIcon variant="subtle" color="gray" aria-label="Bookmark actions">
            <IconDotsVertical size={16} stroke={1.5} />
          </ActionIcon>
        </Tooltip>
      </Menu.Target>
      <Menu.Dropdown>
        <CopyButton value={url} timeout={1500}>
          {({ copied, copy }) => (
            <Menu.Item
              leftSection={
                copied ? <IconCheck size={14} stroke={1.5} /> : <IconCopy size={14} stroke={1.5} />
              }
              onClick={copy}
            >
              {copied ? 'Copied' : 'Copy URL'}
            </Menu.Item>
          )}
        </CopyButton>
        {onFlags ? (
          <>
            <Menu.Item
              leftSection={
                isFavorite ? (
                  <IconStarFilled size={14} stroke={1.5} />
                ) : (
                  <IconStar size={14} stroke={1.5} />
                )
              }
              onClick={() => onFlags({ is_favorite: !isFavorite })}
            >
              {isFavorite ? 'Unfavorite' : 'Favorite'}
            </Menu.Item>
            <Menu.Item
              leftSection={
                isUnread ? (
                  <IconBookmarkFilled size={14} stroke={1.5} />
                ) : (
                  <IconBookmark size={14} stroke={1.5} />
                )
              }
              onClick={() => onFlags({ is_unread: !isUnread })}
            >
              {isUnread ? 'Mark read' : 'Reading list'}
            </Menu.Item>
            <Menu.Item
              leftSection={
                isPinned ? <IconPinFilled size={14} stroke={1.5} /> : <IconPin size={14} stroke={1.5} />
              }
              onClick={() => onFlags({ is_pinned: !isPinned })}
            >
              {isPinned ? 'Unpin' : 'Pin to top'}
            </Menu.Item>
          </>
        ) : null}
        {socialEnabled ? (
          <Menu.Item
            leftSection={<IconMessageCircle size={14} stroke={1.5} />}
            rightSection={<IconHeart size={12} stroke={1.5} />}
            onClick={onOpenSocial}
          >
            Likes & comments
          </Menu.Item>
        ) : null}
        {canWrite ? (
          <>
            <Menu.Item leftSection={<IconPencil size={14} stroke={1.5} />} onClick={onEdit}>
              Edit
            </Menu.Item>
            <Menu.Item color="red" leftSection={<IconTrash size={14} stroke={1.5} />} onClick={onDelete}>
              Delete
            </Menu.Item>
          </>
        ) : null}
      </Menu.Dropdown>
    </Menu>
  )
}
