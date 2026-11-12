import { type ReactElement } from 'react'
import { InfiniteScrollSentinel, LibraryExplorerView } from '@/components'
import { useFolderCounts } from '@/hooks'
import { canWriteCategory } from '@/lib'
import { LibraryGridView } from './LibraryGridView'
import styles from './LibraryPage.module.css'
import { useLibraryPageModel } from './useLibraryPageModel'

export function LibraryResults(): ReactElement {
  const lib = useLibraryPageModel()
  const folderCounts = useFolderCounts(lib.viewMode === 'explorer')

  if (lib.viewMode === 'explorer') {
    return (
      <div className={styles.resultsShell}>
        <LibraryExplorerView
          categories={lib.cats.data || []}
          bookmarks={lib.bookmarks.data || []}
          folderCounts={folderCounts.data}
          searchHits={lib.searchItems}
          searching={lib.searching}
          searchQuery={lib.debouncedQ}
          folderId={lib.folderId}
          sort={lib.sort}
          showHidden={lib.showHidden}
          loading={lib.explorerLoading}
          onFolderChange={lib.onFolderChange}
          onEdit={lib.openEdit}
          onDelete={lib.setDeleteId}
          onMoveToVault={(id) => lib.setMoveTarget({ categoryIds: [], bookmarkIds: [id] })}
          onFlags={lib.onFlags}
          filterFlags={lib.filterFlags}
          onCreate={lib.openCreate}
          emptyDescription={
            lib.scope === 'vault' ? 'Add a bookmark here, or use Move to vault from the Library.' : undefined
          }
          canCreate={lib.canCreateInContext}
          folderReadOnly={!!lib.folderCat && !canWriteCategory(lib.folderCat)}
          viewerUserId={lib.me.data?.user.id}
          selectMode={lib.selectMode}
          selectedIds={lib.selectedIds}
          onToggleSelect={lib.toggleSelect}
          categoryMenuFor={lib.categoryMenuFor}
        />
        {/* At All, Explorer lists folders only, so a further page would load unseen. */}
        {lib.folderId && lib.hasNextPage ? (
          <InfiniteScrollSentinel loading={lib.isFetchingNextPage} onLoadMore={lib.fetchNextPage} />
        ) : null}
      </div>
    )
  }

  return (
    <div className={styles.resultsShell}>
      <LibraryGridView />
    </div>
  )
}
