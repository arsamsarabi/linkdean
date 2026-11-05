import { type ReactElement } from 'react'
import { QueryErrorAlert } from '@/components'
import { useLibraryPageModel } from './useLibraryPageModel'

export function LibraryQueryAlerts(): ReactElement | null {
  const lib = useLibraryPageModel()

  return (
    <>
      {lib.cats.isError ? (
        <QueryErrorAlert error={lib.cats.error} onRetry={() => void lib.cats.refetch()} title="Categories failed" />
      ) : null}
      {lib.bookmarks.isError && !lib.searching ? (
        <QueryErrorAlert
          error={lib.bookmarks.error}
          onRetry={() => void lib.bookmarks.refetch()}
          title="Bookmarks failed"
        />
      ) : null}
      {lib.search.isError && lib.searching ? (
        <QueryErrorAlert error={lib.search.error} onRetry={() => void lib.search.refetch()} title="Search failed" />
      ) : null}
    </>
  )
}
