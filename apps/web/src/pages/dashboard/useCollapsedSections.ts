import { useState } from 'react'

const KEY = 'linkdean.dashboard.collapsed'

function read(): string[] {
  try {
    const raw = window.localStorage.getItem(KEY)
    const parsed: unknown = raw ? JSON.parse(raw) : []
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === 'string') : []
  } catch {
    return []
  }
}

/** Which dashboard sections this browser keeps collapsed. A convenience: lost storage just expands them. */
export function useCollapsedSections(): { isCollapsed: (id: string) => boolean; toggle: (id: string) => void } {
  const [ids, setIds] = useState<string[]>(read)
  function toggle(id: string): void {
    setIds((prev) => {
      const next = prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]
      try {
        window.localStorage.setItem(KEY, JSON.stringify(next.slice(-200)))
      } catch {
        // Storage is optional here.
      }
      return next
    })
  }
  return { isCollapsed: (id) => ids.includes(id), toggle }
}
