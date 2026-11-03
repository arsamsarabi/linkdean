import { useState } from 'react'
import { useMutation, type UseMutationResult } from '@tanstack/react-query'
import { api } from '@/api'
import type { ImportResult } from '@/schemas'

type ImportVars = { format: string; content: string }

export function useImportBookmarks(): UseMutationResult<ImportResult, Error, ImportVars> {
  return useMutation({
    mutationFn: ({ format, content }: ImportVars) => api.import(format, content),
  })
}

export type ExportState = {
  exportBookmarks: (format: string) => Promise<void>
  isExporting: boolean
  exportError: Error | null
  clearExportError: () => void
}

export function useExportBookmarks(): ExportState {
  const [isExporting, setIsExporting] = useState(false)
  const [exportError, setExportError] = useState<Error | null>(null)

  async function exportBookmarks(format: string): Promise<void> {
    setIsExporting(true)
    setExportError(null)
    try {
      const res = await fetch(api.exportUrl(format), {
        credentials: 'include',
      })
      if (!res.ok) {
        throw new Error(res.statusText || 'Export failed')
      }
      const blob = await res.blob()
      const a = document.createElement('a')
      const objectUrl = URL.createObjectURL(blob)
      a.href = objectUrl
      a.download = format === 'html' ? 'linkdean-bookmarks.html' : 'linkdean-export.json'
      a.click()
      URL.revokeObjectURL(objectUrl)
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e))
      setExportError(err)
      throw err
    } finally {
      setIsExporting(false)
    }
  }

  return {
    exportBookmarks,
    isExporting,
    exportError,
    clearExportError: () => setExportError(null),
  }
}
