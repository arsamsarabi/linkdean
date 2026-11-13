import { KeyboardCode, type KeyboardCoordinateGetter } from '@dnd-kit/core'

const FORWARD: string[] = [KeyboardCode.Right, KeyboardCode.Down]
const BACK: string[] = [KeyboardCode.Left, KeyboardCode.Up]

/**
 * Keyboard moves for a sortable list that wraps: each arrow moves one place in list order
 * (→ and ↓ forward, ← and ↑ back). dnd-kit's default picks the nearest item by geometry,
 * which skips places when items differ in width, as pinned bookmarks and sections do.
 */
export function listKeyboardCoordinates(ids: readonly string[]): KeyboardCoordinateGetter {
  return (event, { context: { active, over, collisionRect, droppableRects } }) => {
    const step = FORWARD.includes(event.code) ? 1 : BACK.includes(event.code) ? -1 : 0
    if (step === 0 || !active || !collisionRect) return undefined
    event.preventDefault()
    const from = ids.indexOf(String(over?.id ?? active.id))
    const target = ids[from + step]
    const rect = target === undefined ? undefined : droppableRects.get(target)
    if (!rect) return undefined
    // Past the dragged item, line up the far edges, as the list shifts the others back by its size.
    const after = ids.indexOf(target!) > ids.indexOf(String(active.id))
    return {
      x: rect.left - (after ? collisionRect.width - rect.width : 0),
      y: rect.top - (after ? collisionRect.height - rect.height : 0),
    }
  }
}
