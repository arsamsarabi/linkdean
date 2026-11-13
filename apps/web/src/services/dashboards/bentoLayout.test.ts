import { describe, expect, it } from 'vitest'
import { bentoLayout, keyboardStep, layoutByKey, readingOrder } from './bentoLayout'

const place = (key: string, x: number, y: number, w: number, h: number) => ({ key, layout: { x, y, w, h } })

describe('bentoLayout', () => {
  it('packs sections upward into gaps', () => {
    const l = bentoLayout([place('a', 0, 0, 8, 4), place('b', 8, 0, 4, 10), place('c', 0, 40, 8, 4)])
    expect(layoutByKey(l).get('c')).toEqual({ x: 0, y: 4, w: 8, h: 4 })
  })

  it('shrinks collapsed sections to their header and pulls the rest up', () => {
    const l = bentoLayout([place('a', 0, 0, 12, 10), place('b', 0, 10, 12, 4)], (k) => k === 'a')
    expect(layoutByKey(l).get('a')!.h).toBe(2)
    expect(layoutByKey(l).get('b')!.y).toBe(2)
  })

  it('lists keys top to bottom, then left to right', () => {
    const l = bentoLayout([place('c', 0, 5, 12, 3), place('b', 6, 0, 6, 5), place('a', 0, 0, 6, 5)])
    expect(readingOrder(l)).toEqual(['a', 'b', 'c'])
  })
})

describe('keyboardStep', () => {
  const board = () => bentoLayout([place('a', 0, 0, 12, 4), place('b', 0, 4, 12, 6), place('c', 0, 10, 6, 3)])

  it('moves a section down past the next one, and back up', () => {
    const down = keyboardStep(board(), 'a', 'ArrowDown', false)!
    expect(readingOrder(down)).toEqual(['b', 'a', 'c'])
    const up = keyboardStep(down, 'a', 'ArrowUp', false)!
    expect(readingOrder(up)).toEqual(['a', 'b', 'c'])
  })

  it('moves sideways within the grid', () => {
    const right = keyboardStep(board(), 'c', 'ArrowRight', false)!
    expect(layoutByKey(right).get('c')!.x).toBe(1)
    const atEdge = keyboardStep(board(), 'a', 'ArrowRight', false)!
    expect(layoutByKey(atEdge).get('a')!.x).toBe(0)
  })

  it('resizes with Shift, within the limits', () => {
    const narrower = keyboardStep(board(), 'a', 'ArrowLeft', true)!
    expect(layoutByKey(narrower).get('a')!.w).toBe(11)
    const taller = keyboardStep(board(), 'c', 'ArrowDown', true)!
    expect(layoutByKey(taller).get('c')!.h).toBe(4)
    let l = board()
    for (let i = 0; i < 20; i++) l = keyboardStep(l, 'c', 'ArrowLeft', true)!
    expect(layoutByKey(l).get('c')!.w).toBe(3)
  })

  it('ignores other keys and unknown sections', () => {
    expect(keyboardStep(board(), 'a', 'KeyA', false)).toBeNull()
    expect(keyboardStep(board(), 'zz', 'ArrowDown', false)).toBeNull()
  })
})
