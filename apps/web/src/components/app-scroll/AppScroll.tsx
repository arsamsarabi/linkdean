import { useEffect, useState, type ReactElement, type ReactNode } from 'react'
import { ScrollArea } from '@mantine/core'
import { hasClassicScrollbars, SCROLL_ROOT_ID, ScrollRootContext } from '@/hooks/render'
import styles from './AppScroll.module.css'

const classic = hasClassicScrollbars()

/**
 * Where scrollbars take layout space, the app scrolls inside an overlay scrollbar, so nothing
 * shifts when a page starts to scroll and no empty gutter is left when it does not.
 */
export function AppScroll({ children }: { children: ReactNode }): ReactElement {
  const [viewport, setViewport] = useState<HTMLDivElement | null>(null)

  // Page Down and the arrow keys only scroll a nested scroller that holds focus, so take focus
  // whenever it falls back to the page body.
  useEffect(() => {
    if (!viewport) return
    const claim = (): void => {
      if (document.activeElement === document.body) viewport.focus({ preventScroll: true })
    }
    const onFocusOut = (): void => void setTimeout(claim)
    claim()
    document.addEventListener('focusout', onFocusOut)
    return () => document.removeEventListener('focusout', onFocusOut)
  }, [viewport])

  if (!classic) return <>{children}</>
  return (
    <ScrollArea
      h="100dvh"
      type="hover"
      scrollbars="y"
      viewportRef={setViewport}
      viewportProps={{ id: SCROLL_ROOT_ID, tabIndex: -1 }}
      classNames={{ viewport: styles.viewport, scrollbar: styles.scrollbar }}
    >
      <ScrollRootContext.Provider value={viewport}>{children}</ScrollRootContext.Provider>
    </ScrollArea>
  )
}
