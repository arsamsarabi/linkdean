import { describe, expect, it } from 'vitest'
import { banDurationBody, banEndLabel, emptyBanDuration, validBanReason } from './banForm'

const now = new Date('2026-11-01T12:00:00')

describe('banDurationBody', () => {
  it('marks a permanent ban', () => {
    expect(banDurationBody(emptyBanDuration, now)).toEqual({ permanent: true })
  })

  it('accepts whole days in range', () => {
    expect(banDurationBody({ ...emptyBanDuration, mode: 'days', days: 30 }, now)).toEqual({ days: 30 })
    for (const days of [0, 3651, 1.5, '']) {
      expect(banDurationBody({ ...emptyBanDuration, mode: 'days', days }, now)).toBeNull()
    }
  })

  it('ends a date at the end of that day', () => {
    const body = banDurationBody({ ...emptyBanDuration, mode: 'date', date: '2026-11-02' }, now)
    expect(new Date(body?.ends_at ?? '').getDate()).toBe(2)
  })

  it('accepts today, which still has time left, and refuses the past', () => {
    expect(banDurationBody({ ...emptyBanDuration, mode: 'date', date: '2026-11-01' }, now)).not.toBeNull()
    expect(banDurationBody({ ...emptyBanDuration, mode: 'date', date: '2026-10-31' }, now)).toBeNull()
    expect(banDurationBody({ ...emptyBanDuration, mode: 'date', date: '' }, now)).toBeNull()
  })
})

describe('validBanReason', () => {
  it('needs text within the limit', () => {
    expect(validBanReason('Spam')).toBe(true)
    expect(validBanReason('   ')).toBe(false)
    expect(validBanReason('x'.repeat(501))).toBe(false)
  })
})

describe('banEndLabel', () => {
  it('says permanently without an end', () => {
    expect(banEndLabel(null)).toBe('permanently')
    expect(banEndLabel('2026-12-01T12:00:00Z')).toContain('until ')
  })
})
