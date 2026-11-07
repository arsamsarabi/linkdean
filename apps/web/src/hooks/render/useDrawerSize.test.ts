import { useMediaQuery } from '@mantine/hooks'
import { renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useDrawerSize } from './useDrawerSize'

vi.mock('@mantine/hooks', () => ({
  useMediaQuery: vi.fn(),
}))

describe('useDrawerSize', () => {
  it('is full width on a narrow viewport', () => {
    vi.mocked(useMediaQuery).mockReturnValue(true)
    const { result } = renderHook(() => useDrawerSize())
    expect(result.current).toBe('100%')
  })

  it('keeps the desktop size on a wide viewport', () => {
    vi.mocked(useMediaQuery).mockReturnValue(false)
    const { result } = renderHook(() => useDrawerSize('lg'))
    expect(result.current).toBe('lg')
  })
})
