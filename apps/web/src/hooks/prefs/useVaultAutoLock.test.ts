import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { UserSettings } from '@/schemas'
import { AUTO_LOCK_RETRY_MS, useVaultAutoLock } from './usePrefs'

const settings = { vault_auto_lock_minutes: 1 } as UserSettings

describe('useVaultAutoLock', () => {
  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('tries a failed idle lock again instead of giving up', async () => {
    vi.useFakeTimers()
    vi.spyOn(console, 'error').mockImplementation(() => {})
    const lock = vi.fn().mockRejectedValueOnce(new Error('offline')).mockResolvedValue(undefined)
    renderHook(() => useVaultAutoLock(settings, true, { current: lock }))

    await vi.advanceTimersByTimeAsync(60_000)
    expect(lock).toHaveBeenCalledTimes(1)
    await vi.advanceTimersByTimeAsync(AUTO_LOCK_RETRY_MS)
    expect(lock).toHaveBeenCalledTimes(2)
    await vi.advanceTimersByTimeAsync(AUTO_LOCK_RETRY_MS)
    expect(lock).toHaveBeenCalledTimes(2)
  })
})
