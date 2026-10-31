import type { ReactElement } from 'react'
import { Badge, Group, Text } from '@mantine/core'
import { IconFolderFilled, IconLock, IconShare } from '@tabler/icons-react'
import type { ExplorerFolder } from '@/services'
import styles from './ExplorerFolderTile.module.css'

type Props = {
  folder: ExplorerFolder
  onOpen: (id: string) => void
}

export function ExplorerFolderTile({ folder, onOpen }: Props): ReactElement {
  const countLabel = `${folder.bookmarkCount} bookmark${folder.bookmarkCount === 1 ? '' : 's'}`
  const sharedRo = folder.access === 'read'
  const sharedRw = folder.access === 'write'
  const showBadges = folder.isVault || folder.isHidden || sharedRo || sharedRw

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
            {sharedRo ? (
              <Badge size="xs" variant="outline" color="blue" leftSection={<IconShare size={10} stroke={1.5} />}>
                Shared · RO
              </Badge>
            ) : null}
            {sharedRw ? (
              <Badge size="xs" variant="light" color="blue" leftSection={<IconShare size={10} stroke={1.5} />}>
                Shared · RW
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
