import { type ReactElement } from 'react'
import { SimpleGrid } from '@mantine/core'
import type { CategoryFolderMenuProps } from '@/components/category/CategoryFolderMenu'
import type { Category } from '@/schemas'
import type { ExplorerFolder, ExplorerFolderId } from '@/services'
import { ExplorerFolderTile } from './ExplorerFolderTile'

type CategoryMenuHandlers = Omit<CategoryFolderMenuProps, 'name' | 'access' | 'isVault' | 'className'>

type Props = {
  folders: ExplorerFolder[]
  categoriesById: Map<string, Category>
  onOpen: (id: ExplorerFolderId) => void
  categoryMenuFor?: (cat: Category) => CategoryMenuHandlers | undefined
}

export function ExplorerFolderGrid({ folders, categoriesById, onOpen, categoryMenuFor }: Props): ReactElement {
  return (
    <SimpleGrid cols={{ base: 2, sm: 3, md: 4, lg: 5 }} spacing="md">
      {folders.map((f) => {
        const cat = categoriesById.get(f.id)
        const menu = cat && categoryMenuFor ? categoryMenuFor(cat) : undefined
        return <ExplorerFolderTile key={f.id} folder={f} onOpen={onOpen} menu={menu} />
      })}
    </SimpleGrid>
  )
}
