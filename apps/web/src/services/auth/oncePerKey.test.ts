import { describe, expect, it, vi } from 'vitest'
import { oncePerKey } from './oncePerKey'

describe('oncePerKey', () => {
  it('runs once per key and shares the result', async () => {
    const run = vi.fn().mockResolvedValue('ok')
    const a = oncePerKey('k1', run)
    const b = oncePerKey('k1', run)
    expect(a).toBe(b)
    expect(await b).toBe('ok')
    expect(run).toHaveBeenCalledTimes(1)
  })

  it('runs again for another key', async () => {
    const run = vi.fn().mockResolvedValue('ok')
    await oncePerKey('k2', run)
    await oncePerKey('k3', run)
    expect(run).toHaveBeenCalledTimes(2)
  })
})
