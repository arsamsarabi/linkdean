import { type ReactElement } from 'react'
import { LibraryExplorerView } from '@/components'
import { canWriteCategory } from '@/lib'
import { LibraryGridView } from './LibraryGridView'
import styles from './LibraryPage.module.css'
import { useLibraryPageModel } from './LibraryPageContext'

export function LibraryResults(): ReactElement {
  const lib = useLibraryPageModel()

  if (lib.viewMode === 'explorer') {
    return (
      <div className={styles.resultsShell}>
        <LibraryExplorerView
          categories={lib.cats.data || []}
          bookmarks={lib.bookmarks.data || []}
          searchHits={lib.search.data?.hits || []}
          searching={lib.searching}
          searchQuery={lib.debouncedQ}
          folderId={lib.folderId}
          sort={lib.sort}
          showHidden={lib.showHidden}
          loading={lib.explorerLoading}
          onFolderChange={lib.onFolderChange}
          onEdit={lib.openEdit}
          onDelete={lib.setDeleteId}
          onFlags={lib.onFlags}
          filterFlags={lib.filterFlags}
          onCreate={lib.openCreate}
          canCreate={lib.canCreateInContext}
          folderReadOnly={!!lib.folderCat && !canWriteCategory(lib.folderCat)}
          viewerUserId={lib.me.data?.user.id}
          selectMode={lib.selectMode}
          selectedIds={lib.selectedIds}
          onToggleSelect={lib.toggleSelect}
          categoryMenuFor={lib.categoryMenuFor}
        />
      </div>
    )
  }

  return (
    <div className={styles.resultsShell}>
      <LibraryGridView />
    </div>
  )
}
