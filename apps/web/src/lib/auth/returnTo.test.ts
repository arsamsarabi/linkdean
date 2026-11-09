import { afterEach, describe, expect, it } from 'vitest'
import { rememberReturnTo, takeReturnTo } from './returnTo'

describe('return after sign-in', () => {
  afterEach(() => sessionStorage.clear())

  it('keeps a quick save address once', () => {
    rememberReturnTo('/save?url=https%3A%2F%2Fexample.com')
    expect(takeReturnTo()).toBe('/save?url=https%3A%2F%2Fexample.com')
    expect(takeReturnTo()).toBeNull()
  })

  it('ignores every other page', () => {
    rememberReturnTo('/settings')
    rememberReturnTo('/saved-searches')
    rememberReturnTo('https://evil.example/save')
    expect(takeReturnTo()).toBeNull()
  })
})
