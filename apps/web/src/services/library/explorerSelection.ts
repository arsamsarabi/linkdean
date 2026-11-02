import type { SortKey } from '@/lib'
import type { Bookmark, Category } from '@/schemas'
import { listFolderContents, type ExplorerFolderId } from './explorerLibrary'
import { filterAndSortLibraryItems, type FlagFilter, type LibraryListItem } from './libraryList'

type Input = {
  searching: boolean
  searchHits: readonly LibraryListItem[]
  folderId: ExplorerFolderId | undefined
  categories: readonly Category[]
  bookmarks: readonly Bookmark[]
  showHidden: boolean
  filterFlags: FlagFilter[]
  sort: SortKey
}

/** The items Explorer shows right now, which is all that bulk select may reach. */
export function explorerSelectableItems({
  searching,
  searchHits,
  folderId,
  categories,
  bookmarks,
  showHidden,
  filterFlags,
  sort,
}: Input): LibraryListItem[] {
  const items = searching ? searchHits : listFolderContents(folderId, categories, bookmarks, showHidden).bookmarks
  return filterAndSortLibraryItems({ items: [...items], filterCats: [], filterTags: [], filterFlags, sort })
}
