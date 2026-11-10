import { createContext, useContext, useEffect, useState } from 'react'

export const SCROLL_ROOT_ID = 'ld-scroll-root'

/** The element the app scrolls in, or null when the window scrolls. */
export const ScrollRootContext = createContext<HTMLElement | null>(null)

export function useScrollRoot(): HTMLElement | null {
  return useContext(ScrollRootContext)
}

/**
 * True when the system draws scrollbars that take layout space. Overlay scrollbars (phones, macOS
 * "when scrolling") take none, so the window can keep scrolling there.
 */
// ponytail: measured once; switching the OS scrollbar setting needs a reload to take effect.
export function hasClassicScrollbars(): boolean {
  if (typeof document === 'undefined') return false
  const probe = document.createElement('div')
  probe.style.cssText = 'position:absolute;top:-9999px;width:100px;height:100px;overflow:scroll'
  document.body.appendChild(probe)
  const width = probe.offsetWidth - probe.clientWidth
  probe.remove()
  return width > 0
}

/** How far the app has scrolled down, from the scroll root or the window. */
export function useScrollY(): number {
  const root = useScrollRoot()
  const [y, setY] = useState(0)
  useEffect(() => {
    const read = (): void => setY(root ? root.scrollTop : window.scrollY)
    const target = root ?? window
    read()
    target.addEventListener('scroll', read, { passive: true })
    return () => target.removeEventListener('scroll', read)
  }, [root])
  return y
}
