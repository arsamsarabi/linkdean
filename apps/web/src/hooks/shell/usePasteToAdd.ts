import { useEffect } from 'react'
import { useNavigate, useRouterState } from '@tanstack/react-router'
import { asWebUrl, buildLibrarySearch } from '@/lib'

function isEditable(el: EventTarget | null): boolean {
  if (!(el instanceof HTMLElement)) return false
  return el.isContentEditable || el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT'
}

/**
 * Paste a web address anywhere outside a text field, and the new-bookmark form opens with it
 * filled in. On the library the open folder is kept, so the bookmark lands where you are.
 */
export function usePasteToAdd(enabled: boolean): void {
  const nav = useNavigate()
  const location = useRouterState({ select: (s) => s.location })

  useEffect(() => {
    if (!enabled) return
    function onPaste(e: ClipboardEvent): void {
      if (isEditable(e.target) || isEditable(document.activeElement)) return
      // A dialog is open: leave the paste to it.
      if (document.querySelector('[role="dialog"]')) return
      const url = asWebUrl(e.clipboardData?.getData('text/plain'))
      if (!url) return
      e.preventDefault()
      const onLibrary = location.pathname.startsWith('/library')
      const folder = onLibrary ? (location.search as { folder?: string }).folder : undefined
      void nav({ to: '/library', search: buildLibrarySearch({ addUrl: url, folder }) })
    }
    document.addEventListener('paste', onPaste)
    return () => document.removeEventListener('paste', onPaste)
  }, [enabled, location, nav])
}
