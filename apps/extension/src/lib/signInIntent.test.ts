import { beforeEach, describe, expect, it } from 'vitest'
import { fakeBrowser } from 'wxt/testing/fake-browser'
import { intentMatches, SIGN_IN_INTENT_MAX_AGE_MS, signInIntentItem, takeSignInIntent } from './signIn'

const instance = 'https://app.linkdean.app'
const pattern = 'https://app.linkdean.app/*'

describe('sign-in started from the permission grant', () => {
  beforeEach(() => {
    fakeBrowser.reset()
  })

  it('matches a recent intent for a granted site only', () => {
    const intent = { instance, at: 1_000 }
    expect(intentMatches(intent, [pattern], 1_000 + SIGN_IN_INTENT_MAX_AGE_MS - 1)).toBe(true)
    expect(intentMatches(intent, [pattern], 1_000 + SIGN_IN_INTENT_MAX_AGE_MS)).toBe(false)
    expect(intentMatches(intent, ['https://other.example/*'], 1_001)).toBe(false)
    expect(intentMatches(null, [pattern], 1_001)).toBe(false)
  })

  it('hands the intent out once when the grant and the popup both ask', async () => {
    await signInIntentItem.setValue({ instance, at: Date.now() })
    const results = await Promise.all([takeSignInIntent([pattern]), takeSignInIntent([pattern])])
    expect(results.sort()).toEqual([instance, null].sort())
    expect(await signInIntentItem.getValue()).toBeNull()
  })

  it('leaves an intent for another site in place', async () => {
    await signInIntentItem.setValue({ instance, at: Date.now() })
    expect(await takeSignInIntent(['https://other.example/*'])).toBeNull()
    expect(await signInIntentItem.getValue()).toEqual(expect.objectContaining({ instance }))
  })
})
