import { type ReactElement, type ReactNode } from 'react'
import ReactGridLayout, { useContainerWidth, verticalCompactor, type Layout } from 'react-grid-layout'
import 'react-grid-layout/css/styles.css'
import { DASHBOARD_GRID_COLS } from '@/schemas'
import { BENTO_GAP, BENTO_MIN_WIDTH, BENTO_ROW_HEIGHT, readingOrder } from '@/services'
import styles from './BentoGrid.module.css'

type Props = {
  /** The packed layout, one item per section key. */
  layout: Layout
  /** Drag by `.dash-drag-handle` and resize from the corner and edges. */
  editing?: boolean
  onLayoutChange?: (layout: Layout) => void
  renderItem: (key: string) => ReactNode
}

/**
 * The dashboard's bento grid: sections placed and sized on 12 columns, packed upward. On a
 * narrow board (phones) they stack in reading order at their natural height instead.
 */
export function BentoGrid({ layout, editing = false, onLayoutChange, renderItem }: Props): ReactElement {
  const { width, containerRef, mounted } = useContainerWidth()
  // Children in reading order, so Tab, screen readers and the arrow keys follow the board.
  const keys = readingOrder(layout)

  return (
    <div ref={containerRef} data-bento={editing ? 'editing' : 'view'}>
      {mounted && width < BENTO_MIN_WIDTH ? (
        <div className={styles.stack}>
          {keys.map((key) => (
            <div key={key} className={styles.tile}>
              {renderItem(key)}
            </div>
          ))}
        </div>
      ) : mounted ? (
        <ReactGridLayout
          className={styles.grid}
          width={width}
          layout={layout}
          compactor={verticalCompactor}
          gridConfig={{
            cols: DASHBOARD_GRID_COLS,
            rowHeight: BENTO_ROW_HEIGHT,
            margin: [BENTO_GAP, BENTO_GAP],
            containerPadding: [0, 0],
          }}
          dragConfig={{ enabled: editing, handle: '.dash-drag-handle', threshold: 4 }}
          resizeConfig={{ enabled: editing, handles: ['se', 'e', 's'] }}
          onLayoutChange={onLayoutChange}
        >
          {keys.map((key) => (
            <div key={key} className={styles.tile}>
              {renderItem(key)}
            </div>
          ))}
        </ReactGridLayout>
      ) : null}
    </div>
  )
}
