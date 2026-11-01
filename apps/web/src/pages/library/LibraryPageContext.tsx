import { createContext, useContext, type ReactElement, type ReactNode } from 'react'
import { useLibraryPage } from '@/hooks'

type LibraryPageModel = ReturnType<typeof useLibraryPage>

const LibraryPageContext = createContext<LibraryPageModel | null>(null)

export function LibraryPageProvider({ children }: { children: ReactNode }): ReactElement {
  const lib = useLibraryPage()
  return <LibraryPageContext.Provider value={lib}>{children}</LibraryPageContext.Provider>
}

/** Library page session — call in colocated children instead of prop-drilling the composition root. */
export function useLibraryPageModel(): LibraryPageModel {
  const ctx = useContext(LibraryPageContext)
  if (!ctx) throw new Error('useLibraryPageModel requires LibraryPageProvider')
  return ctx
}
