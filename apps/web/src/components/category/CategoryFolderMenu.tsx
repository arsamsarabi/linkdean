import type { MouseEvent, ReactElement, ReactNode } from 'react'
import { ActionIcon, Menu } from '@mantine/core'
import { IconCopy, IconDots, IconLock, IconMessageCircle, IconPencil, IconShare, IconTrash } from '@tabler/icons-react'
import type { CategoryAccess } from '@/schemas'

export type CategoryFolderMenuProps = {
  name: string
  access: CategoryAccess
  isVault: boolean
  onEdit?: () => void
  onShare?: () => void
  onSocial?: () => void
  onFork?: () => void
  onDelete?: () => void
  onMoveToVault?: () => void
  forkPending?: boolean
  /** Wrapper class for positioning (e.g. absolute corner). */
  className?: string
}

function categoryFolderMenuVisible(props: CategoryFolderMenuProps): boolean {
  const isOwner = props.access === 'owner'
  const canWrite = isOwner || props.access === 'write'
  return (
    (canWrite && !!props.onEdit) ||
    (!props.isVault && !!props.onFork) ||
    (isOwner && !props.isVault && !!props.onShare) ||
    !!props.onSocial ||
    (isOwner && (!!props.onDelete || !!props.onMoveToVault))
  )
}

export function CategoryFolderMenu(props: CategoryFolderMenuProps): ReactElement | null {
  if (!categoryFolderMenuVisible(props)) return null

  const { name, access, isVault, onEdit, onShare, onSocial, onFork, onDelete, onMoveToVault, forkPending, className } =
    props
  const isOwner = access === 'owner'
  const canWrite = isOwner || access === 'write'

  function stop(e: MouseEvent): void {
    e.stopPropagation()
    e.preventDefault()
  }

  const target: ReactNode = (
    <ActionIcon size="sm" variant="subtle" aria-label={`Actions for ${name}`} onClick={stop}>
      <IconDots size={16} stroke={1.5} />
    </ActionIcon>
  )

  return (
    <div className={className} onClick={stop}>
      <Menu shadow="md" width={180} position="bottom-end" withinPortal>
        <Menu.Target>{target}</Menu.Target>
        <Menu.Dropdown>
          {canWrite && onEdit ? (
            <Menu.Item leftSection={<IconPencil size={14} stroke={1.5} />} onClick={onEdit}>
              Edit
            </Menu.Item>
          ) : null}
          {!isVault && onFork ? (
            <Menu.Item leftSection={<IconCopy size={14} stroke={1.5} />} disabled={forkPending} onClick={onFork}>
              Duplicate
            </Menu.Item>
          ) : null}
          {isOwner && !isVault && onShare ? (
            <Menu.Item leftSection={<IconShare size={14} stroke={1.5} />} onClick={onShare}>
              Share
            </Menu.Item>
          ) : null}
          {isOwner && onMoveToVault ? (
            <Menu.Item leftSection={<IconLock size={14} stroke={1.5} />} onClick={onMoveToVault}>
              Move to vault
            </Menu.Item>
          ) : null}
          {onSocial ? (
            <Menu.Item leftSection={<IconMessageCircle size={14} stroke={1.5} />} onClick={onSocial}>
              Likes & comments
            </Menu.Item>
          ) : null}
          {isOwner && onDelete ? (
            <>
              <Menu.Divider />
              <Menu.Item color="red" leftSection={<IconTrash size={14} stroke={1.5} />} onClick={onDelete}>
                Delete
              </Menu.Item>
            </>
          ) : null}
        </Menu.Dropdown>
      </Menu>
    </div>
  )
}
