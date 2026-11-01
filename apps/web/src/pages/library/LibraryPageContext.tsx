import { type ReactElement, type ReactNode } from 'react'
import { useLibraryPage, type LibraryScope } from '@/hooks'
import { LibraryPageContext } from './useLibraryPageModel'

export function LibraryPageProvider({ scope, children }: { scope: LibraryScope; children: ReactNode }): ReactElement {
  const lib = useLibraryPage(scope)
  return <LibraryPageContext.Provider value={lib}>{children}</LibraryPageContext.Provider>
}
