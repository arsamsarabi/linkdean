import type { ReactElement } from 'react'
import type { CategoryFolderMenuProps } from '@/components/category/CategoryFolderMenu'
import type { ExplorerFolder } from '@/services'
import { FolderTile } from './FolderTile'

type MenuHandlers = Omit<CategoryFolderMenuProps, 'name' | 'access' | 'isVault' | 'className'>

type Props = {
  folder: ExplorerFolder
  onOpen: (id: string) => void
  menu?: MenuHandlers
}

export function ExplorerFolderTile({ folder, onOpen, menu }: Props): ReactElement {
  const countLabel = `${folder.bookmarkCount} bookmark${folder.bookmarkCount === 1 ? '' : 's'}`
  return (
    <FolderTile
      id={folder.id}
      name={folder.name}
      countLabel={countLabel}
      onOpen={onOpen}
      isVault={folder.isVault}
      isHidden={folder.isHidden}
      icon={folder.icon}
      access={folder.access}
      sharedOut={folder.sharedOut}
      sharedWith={folder.sharedWith}
      ownerDisplayName={folder.ownerDisplayName}
      deletedAt={folder.deletedAt}
      menu={menu}
    />
  )
}
