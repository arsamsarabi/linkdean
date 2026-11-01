import { createContext, useContext } from 'react'
import type { useLibraryPage } from '@/hooks'

type LibraryPageModel = ReturnType<typeof useLibraryPage>

export const LibraryPageContext = createContext<LibraryPageModel | null>(null)

/** Library page session — call in colocated children instead of prop-drilling the composition root. */
export function useLibraryPageModel(): LibraryPageModel {
  const ctx = useContext(LibraryPageContext)
  if (!ctx) throw new Error('useLibraryPageModel requires LibraryPageProvider')
  return ctx
}
