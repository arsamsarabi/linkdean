import { type ReactElement } from 'react'
import { Stack } from '@mantine/core'
import { LibraryBulkBar } from './LibraryBulkBar'
import { LibraryFab } from './LibraryFab'
import { LibraryPageProvider } from './LibraryPageContext'
import { LibraryPageHeader } from './LibraryPageHeader'
import { LibraryPageModals } from './LibraryPageModals'
import { LibraryQueryAlerts } from './LibraryQueryAlerts'
import { LibraryResults } from './LibraryResults'
import { LibrarySearchToolbar } from './LibrarySearchToolbar'

export function LibraryPage(): ReactElement {
  return (
    <LibraryPageProvider>
      <Stack gap="xl">
        <LibraryPageHeader />
        <LibraryBulkBar />
        <LibraryQueryAlerts />
        <LibrarySearchToolbar />
        <LibraryResults />
        <LibraryPageModals />
        <LibraryFab />
      </Stack>
    </LibraryPageProvider>
  )
}
