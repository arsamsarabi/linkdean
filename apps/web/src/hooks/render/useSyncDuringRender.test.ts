import { useState } from 'react'
import { cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useSyncDuringRender } from './useSyncDuringRender'

describe('useSyncDuringRender', () => {
  afterEach(cleanup)

  it('syncs on first render and only again when a dep changes', () => {
    const sync = vi.fn()
    const { rerender } = renderHook(({ dep }) => useSyncDuringRender([dep], sync), { initialProps: { dep: 'a' } })
    expect(sync).toHaveBeenCalledTimes(1)
    rerender({ dep: 'a' })
    expect(sync).toHaveBeenCalledTimes(1)
    rerender({ dep: 'b' })
    expect(sync).toHaveBeenCalledTimes(2)
  })

  it('returns the synced state from the same commit', () => {
    const { result, rerender } = renderHook(
      ({ dep }) => {
        const [value, setValue] = useState('')
        useSyncDuringRender([dep], () => setValue(dep))
        return value
      },
      { initialProps: { dep: 'a' } }
    )
    expect(result.current).toBe('a')
    rerender({ dep: 'b' })
    expect(result.current).toBe('b')
  })
})
