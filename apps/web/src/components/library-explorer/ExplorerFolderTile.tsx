import type { ReactElement } from 'react'
import { Badge, Group, Text } from '@mantine/core'
import { IconFolderFilled, IconLock } from '@tabler/icons-react'
import { ShareBadge } from '@/components/share-badge'
import type { ExplorerFolder } from '@/services'
import styles from './ExplorerFolderTile.module.css'

type Props = {
  folder: ExplorerFolder
  onOpen: (id: string) => void
}

export function ExplorerFolderTile({ folder, onOpen }: Props): ReactElement {
  const countLabel = `${folder.bookmarkCount} bookmark${folder.bookmarkCount === 1 ? '' : 's'}`
  const shared =
    folder.access === 'read' || folder.access === 'write' ? folder.access : null
  const showOutgoing = !shared && !!folder.sharedOut
  const pendingDelete = !!folder.deletedAt
  const showBadges = folder.isVault || folder.isHidden || !!shared || showOutgoing || pendingDelete

  return (
    <button
      type="button"
      className={styles.tile}
      onClick={() => onOpen(folder.id)}
      aria-label={`Open folder ${folder.name}, ${countLabel}`}
    >
      <IconFolderFilled className={styles.icon} size={40} aria-hidden />
      <div className={styles.name}>
        <Text size="sm" fw={600} lineClamp={2}>
          {folder.name}
        </Text>
        <Text size="xs" c="dimmed" mt={2}>
          {countLabel}
        </Text>
        {showBadges ? (
          <Group gap={4} justify="center" mt={6}>
            {shared ? <ShareBadge access={shared} ownerLabel={folder.ownerDisplayName} /> : null}
            {showOutgoing ? <ShareBadge outgoing /> : null}
            {pendingDelete ? (
              <Badge size="xs" variant="outline" color="red" tt="none">
                Pending delete
              </Badge>
            ) : null}
            {folder.isVault ? (
              <Badge size="xs" variant="light" color="grape" leftSection={<IconLock size={10} stroke={1.5} />}>
                Vault
              </Badge>
            ) : null}
            {folder.isHidden ? (
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
