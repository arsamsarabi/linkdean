import { type ReactElement, type ReactNode } from 'react'
import { useLibraryPage } from '@/hooks'
import { LibraryPageContext } from './useLibraryPageModel'

export function LibraryPageProvider({ children }: { children: ReactNode }): ReactElement {
  const lib = useLibraryPage()
  return <LibraryPageContext.Provider value={lib}>{children}</LibraryPageContext.Provider>
}
