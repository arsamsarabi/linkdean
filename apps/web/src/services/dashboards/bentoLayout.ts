import { cloneLayout, getLayoutItem, moveElement, verticalCompactor, type Layout } from 'react-grid-layout/core'
import {
  DASHBOARD_GRID_COLS,
  DASHBOARD_SECTION_MAX_H,
  DASHBOARD_SECTION_MIN_H,
  DASHBOARD_SECTION_MIN_W,
  type DashboardSectionDisplay,
  type DashboardSectionLayout,
} from '@/schemas'

/** Grid row height and gap in pixels. A section `h` rows tall is h × 36 + (h − 1) × 24 px. */
export const BENTO_ROW_HEIGHT = 36
export const BENTO_GAP = 24
/** Below this board width the sections stack in one column at their natural height. */
export const BENTO_MIN_WIDTH = 720
/** A row far below the others: the grid packs a new section up under them. */
export const BOTTOM_ROW = 9999
/** A collapsed section shows only its header. */
export const COLLAPSED_ROWS = 2

/** A new section's height, by how it shows its bookmarks. */
export function defaultSectionHeight(display: DashboardSectionDisplay): number {
  return display === 'icons' ? 5 : display === 'small' ? 8 : 11
}

type Placed = { key: string; layout: DashboardSectionLayout }

/** The grid layout for these sections, packed upward. Collapsed ones shrink to their header. */
export function bentoLayout(sections: readonly Placed[], collapsed: (key: string) => boolean = () => false): Layout {
  const items = sections.map(({ key, layout }) => ({
    i: key,
    ...layout,
    h: collapsed(key) ? COLLAPSED_ROWS : layout.h,
    minW: DASHBOARD_SECTION_MIN_W,
    minH: DASHBOARD_SECTION_MIN_H,
    maxH: DASHBOARD_SECTION_MAX_H,
  }))
  return verticalCompactor.compact(items, DASHBOARD_GRID_COLS)
}

/** Section keys in reading order: top to bottom, then left to right. */
export function readingOrder(layout: Layout): string[] {
  return [...layout].sort((a, b) => a.y - b.y || a.x - b.x).map((l) => l.i)
}

/** The x, y, w and h of each item, keyed by section key. */
export function layoutByKey(layout: Layout): Map<string, DashboardSectionLayout> {
  return new Map(layout.map((l) => [l.i, { x: l.x, y: l.y, w: l.w, h: l.h }]))
}

const clamp = (v: number, lo: number, hi: number): number => Math.min(hi, Math.max(lo, v))

/**
 * One keyboard step on the board. Arrows move a section one column or one place up or down
 * (pushing the neighbour the other way); with Shift they make it a column narrower or wider,
 * or a row shorter or taller. Returns the packed layout, or null for any other key.
 */
export function keyboardStep(layout: Layout, key: string, code: string, resize: boolean): Layout | null {
  const next = cloneLayout(layout)
  const item = getLayoutItem(next, key)
  if (!item) return null
  if (resize) {
    if (code === 'ArrowLeft') item.w = clamp(item.w - 1, DASHBOARD_SECTION_MIN_W, DASHBOARD_GRID_COLS)
    else if (code === 'ArrowRight') {
      item.w = clamp(item.w + 1, DASHBOARD_SECTION_MIN_W, DASHBOARD_GRID_COLS)
      item.x = Math.min(item.x, DASHBOARD_GRID_COLS - item.w)
    } else if (code === 'ArrowUp') item.h = clamp(item.h - 1, DASHBOARD_SECTION_MIN_H, DASHBOARD_SECTION_MAX_H)
    else if (code === 'ArrowDown') item.h = clamp(item.h + 1, DASHBOARD_SECTION_MIN_H, DASHBOARD_SECTION_MAX_H)
    else return null
    return verticalCompactor.compact(next, DASHBOARD_GRID_COLS)
  }
  let x = item.x
  let y = item.y
  if (code === 'ArrowLeft') x = Math.max(0, item.x - 1)
  else if (code === 'ArrowRight') x = Math.min(DASHBOARD_GRID_COLS - item.w, item.x + 1)
  else if (code === 'ArrowUp') {
    // Jump onto the nearest section above that shares a column, so the two swap.
    const above = next
      .filter((o) => o.i !== key && o.y + o.h <= item.y && o.x < item.x + item.w && item.x < o.x + o.w)
      .sort((a, b) => b.y + b.h - (a.y + a.h))[0]
    if (!above) return null
    y = above.y
  } else if (code === 'ArrowDown') {
    const below = next
      .filter((o) => o.i !== key && o.y >= item.y + item.h && o.x < item.x + item.w && item.x < o.x + o.w)
      .sort((a, b) => a.y - b.y)[0]
    if (!below) return null
    y = below.y + below.h - item.h + 1
  } else return null
  const moved = moveElement(next, item, x, y, true, false, 'vertical', DASHBOARD_GRID_COLS)
  return verticalCompactor.compact(moved, DASHBOARD_GRID_COLS)
}
