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
  // ponytail: Explorer only loads the open folder's bookmarks, so a tile's count is unknown.
  // Upgrade: per-folder counts from the categories API.
  return (
    <FolderTile
      id={folder.id}
      name={folder.name}
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
