import { afterEach, describe, expect, it } from 'vitest'
import { applyKeyboardHints } from './usePrefs'

describe('applyKeyboardHints', () => {
  afterEach(() => applyKeyboardHints(true))

  it('marks the page when hints are off and clears the mark when they are on', () => {
    applyKeyboardHints(false)
    expect(document.documentElement.dataset.keyHints).toBe('off')
    applyKeyboardHints(true)
    expect(document.documentElement.dataset.keyHints).toBeUndefined()
  })
})
