import { type ReactElement } from 'react'
import { Stack } from '@mantine/core'
import type { LibraryScope } from '@/hooks'
import { LibraryBulkBar } from './LibraryBulkBar'
import { LibraryFab } from './LibraryFab'
import { LibraryPageProvider } from './LibraryPageContext'
import { LibraryPageHeader } from './LibraryPageHeader'
import { LibraryPageModals } from './LibraryPageModals'
import { LibraryQueryAlerts } from './LibraryQueryAlerts'
import { LibraryResults } from './LibraryResults'
import { LibrarySearchToolbar } from './LibrarySearchToolbar'
import { VaultsBar } from './VaultsBar'

export function LibraryPage({ scope = 'library' }: { scope?: LibraryScope }): ReactElement {
  return (
    <LibraryPageProvider scope={scope}>
      <Stack gap="xl">
        <LibraryPageHeader />
        {scope === 'vault' ? <VaultsBar /> : null}
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
