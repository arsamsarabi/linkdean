import type { ReactElement } from 'react'
import { Badge, Group, Text } from '@mantine/core'
import { IconLock } from '@tabler/icons-react'
import { CategoryFolderMenu, type CategoryFolderMenuProps } from '@/components/category/CategoryFolderMenu'
import { ShareBadge } from '@/components/share-badge'
import { CategoryIcon } from '@/lib'
import type { CategoryAccess } from '@/schemas'
import styles from './FolderTile.module.css'

export type FolderTileProps = {
  id: string
  name: string
  /** Omitted when the count is unknown. */
  countLabel?: string
  onOpen: (id: string) => void
  isVault?: boolean
  isHidden?: boolean
  icon?: string | null
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
  icon,
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
  const menuProps = menu && access ? { name, access, isVault, className: styles.menu, ...menu } : null

  // The menu is a sibling of the tile button: a button may not contain another button.
  return (
    <div className={styles.wrap}>
      <button
        type="button"
        className={styles.tile}
        onClick={() => onOpen(id)}
        aria-label={countLabel ? `Open folder ${name}, ${countLabel}` : `Open folder ${name}`}
      >
        <CategoryIcon icon={icon} className={styles.icon} size={40} aria-hidden />
        <div className={styles.name}>
          <Text size="sm" fw={600} lineClamp={2}>
            {name}
          </Text>
          {countLabel ? (
            <Text size="xs" c="dimmed" mt={2}>
              {countLabel}
            </Text>
          ) : null}
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
      {menuProps ? <CategoryFolderMenu {...menuProps} /> : null}
    </div>
  )
}
