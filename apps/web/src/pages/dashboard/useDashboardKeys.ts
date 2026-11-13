import { useEffect, type RefObject } from 'react'

function typing(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  return target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)
}

function linkIn(tile: Element | undefined): HTMLAnchorElement | null {
  return tile?.querySelector<HTMLAnchorElement>('a[href]') ?? null
}

/**
 * Keyboard use of the dashboard inside `root`:
 * - 1 to 9 open that bookmark of the highlighted section (the first section until one is hovered or focused);
 * - with a bookmark focused, Left and Right move along its section, Up and Down to the section before or after.
 */
export function useDashboardKeys(root: RefObject<HTMLElement | null>, enabled: boolean): void {
  useEffect(() => {
    if (!enabled) return
    function onKey(e: KeyboardEvent): void {
      const el = root.current
      if (!el || e.defaultPrevented || e.metaKey || e.ctrlKey || e.altKey || typing(e.target)) return
      const sections = Array.from(el.querySelectorAll<HTMLElement>('[data-dash-section]'))
      if (sections.length === 0) return

      if (/^[1-9]$/.test(e.key)) {
        const section = sections.find((s) => s.hasAttribute('data-active')) ?? sections[0]
        const tiles = section?.querySelectorAll('[data-dash-tile]')
        const link = linkIn(tiles?.[Number(e.key) - 1])
        if (link) {
          e.preventDefault()
          link.click()
        }
        return
      }

      if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) return
      const current = (e.target as HTMLElement | null)?.closest('[data-dash-tile]')
      const section = current?.closest<HTMLElement>('[data-dash-section]')
      if (!current || !section) return
      const tiles = Array.from(section.querySelectorAll('[data-dash-tile]'))
      const at = tiles.indexOf(current)
      let next: HTMLAnchorElement | null = null
      if (e.key === 'ArrowLeft') next = linkIn(tiles[at - 1])
      if (e.key === 'ArrowRight') next = linkIn(tiles[at + 1])
      if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
        const step = e.key === 'ArrowUp' ? -1 : 1
        for (let i = sections.indexOf(section) + step; i >= 0 && i < sections.length; i += step) {
          next = linkIn(sections[i]!.querySelector('[data-dash-tile]') ?? undefined)
          if (next) break
        }
      }
      if (next) {
        e.preventDefault()
        next.focus()
      }
    }
    document.addEventListener('keydown', onKey)
    return () => document.removeEventListener('keydown', onKey)
  }, [root, enabled])
}
