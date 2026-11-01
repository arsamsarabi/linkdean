import { type ReactElement } from 'react'
import { Alert, Button, Group, Text } from '@mantine/core'
import { QueryErrorAlert } from '@/components'
import { useLibraryPageModel } from './LibraryPageContext'

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
      {lib.searching && lib.searchCatchUpCount > 0 ? (
        <Alert color="yellow" title="Search index catching up" withCloseButton={false}>
          <Group justify="space-between" align="center" wrap="wrap" gap="sm">
            <Text size="sm">
              Showing {lib.searchCatchUpCount} local match
              {lib.searchCatchUpCount === 1 ? '' : 'es'} while Meilisearch finishes indexing.
            </Text>
            <Button
              size="compact-sm"
              variant="light"
              loading={lib.search.isFetching}
              onClick={() => void lib.search.refetch()}
            >
              Retry search
            </Button>
          </Group>
        </Alert>
      ) : null}
    </>
  )
}
