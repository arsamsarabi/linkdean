import type { ReactElement } from 'react'
import { Badge, Group, Text } from '@mantine/core'
import { IconFolderFilled, IconLock } from '@tabler/icons-react'
import {
  CategoryFolderMenu,
  type CategoryFolderMenuProps,
} from '@/components/category/CategoryFolderMenu'
import { ShareBadge } from '@/components/share-badge'
import type { CategoryAccess } from '@/schemas'
import styles from './FolderTile.module.css'

export type FolderTileProps = {
  id: string
  name: string
  countLabel: string
  onOpen: (id: string) => void
  isVault?: boolean
  isHidden?: boolean
  access?: CategoryAccess
  sharedOut?: boolean
  sharedWith?: readonly string[]
  ownerDisplayName?: string | null
  deletedAt?: string | null
  /** When set, shows the category ⋮ menu (same as Settings → Categories). */
  menu?: Omit<CategoryFolderMenuProps, 'name' | 'access' | 'isVault' | 'className'>
}

export function FolderTile({
  id,
  name,
  countLabel,
  onOpen,
  isVault = false,
  isHidden = false,
  access,
  sharedOut = false,
  sharedWith,
  ownerDisplayName,
  deletedAt,
  menu,
}: FolderTileProps): ReactElement {
  const shared = access === 'read' || access === 'write' ? access : null
  const showOutgoing = !shared && !!sharedOut
  const pendingDelete = !!deletedAt
  const showBadges = isVault || isHidden || !!shared || showOutgoing || pendingDelete
  const menuProps =
    menu && access
      ? { name, access, isVault, className: styles.menu, ...menu }
      : null

  return (
    <button
      type="button"
      className={styles.tile}
      onClick={() => onOpen(id)}
      aria-label={`Open folder ${name}, ${countLabel}`}
    >
      {menuProps ? <CategoryFolderMenu {...menuProps} /> : null}
      <IconFolderFilled className={styles.icon} size={40} aria-hidden />
      <div className={styles.name}>
        <Text size="sm" fw={600} lineClamp={2}>
          {name}
        </Text>
        <Text size="xs" c="dimmed" mt={2}>
          {countLabel}
        </Text>
        {showBadges ? (
          <Group gap={4} justify="center" mt={6}>
            {shared ? <ShareBadge access={shared} ownerLabel={ownerDisplayName} /> : null}
            {showOutgoing ? <ShareBadge outgoing sharedWith={sharedWith} /> : null}
            {pendingDelete ? (
              <Badge size="xs" variant="outline" color="red" tt="none">
                Pending delete
              </Badge>
            ) : null}
            {isVault ? (
              <Badge size="xs" variant="light" color="grape" leftSection={<IconLock size={10} stroke={1.5} />}>
                Vault
              </Badge>
            ) : null}
            {isHidden ? (
              <Badge size="xs" variant="outline" color="gray">
                Hidden
              </Badge>
            ) : null}
          </Group>
        ) : null}
      </div>
    </button>
  )
}
