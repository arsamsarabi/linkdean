import { useRef } from 'react'
import { cleanup, fireEvent, render } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useDashboardKeys } from './useDashboardKeys'

const clicks = vi.fn()

function Board(): React.ReactElement {
  const root = useRef<HTMLDivElement>(null)
  useDashboardKeys(root, true)
  const tile = (id: string) => (
    <div data-dash-tile key={id}>
      <a
        href={`https://${id}.example`}
        onClick={(e) => {
          e.preventDefault()
          clicks(id)
        }}
      >
        {id}
      </a>
    </div>
  )
  return (
    <div ref={root}>
      <section data-dash-section>{['a1', 'a2', 'a3'].map(tile)}</section>
      <section data-dash-section data-active>
        {['b1', 'b2'].map(tile)}
      </section>
      <input aria-label="filter" />
    </div>
  )
}

describe('useDashboardKeys', () => {
  afterEach(() => {
    cleanup()
    clicks.mockReset()
  })

  it('opens the nth bookmark of the highlighted section', () => {
    render(<Board />)
    fireEvent.keyDown(document.body, { key: '2' })
    expect(clicks).toHaveBeenCalledWith('b2')
    fireEvent.keyDown(document.body, { key: '9' })
    expect(clicks).toHaveBeenCalledTimes(1)
  })

  it('leaves typing alone', () => {
    const { getByLabelText } = render(<Board />)
    fireEvent.keyDown(getByLabelText('filter'), { key: '1' })
    expect(clicks).not.toHaveBeenCalled()
  })

  it('moves focus along a section and between sections', () => {
    const { getByText } = render(<Board />)
    getByText('a1').focus()
    fireEvent.keyDown(getByText('a1'), { key: 'ArrowRight' })
    expect(document.activeElement).toBe(getByText('a2'))
    fireEvent.keyDown(getByText('a2'), { key: 'ArrowDown' })
    expect(document.activeElement).toBe(getByText('b1'))
    fireEvent.keyDown(getByText('b1'), { key: 'ArrowUp' })
    expect(document.activeElement).toBe(getByText('a1'))
  })
})
