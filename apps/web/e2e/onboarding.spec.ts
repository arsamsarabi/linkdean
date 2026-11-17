import { test } from '@playwright/test'

test.describe('Onboarding', () => {
  test.skip('ONB-01 @onboarding @smoke: first-admin setup completes and lands in app — assert stepper reaches the end and the app shell loads', async () => {})
  test.skip('ONB-02 @onboarding @p1: admin step validates email and password confirmation — assert errors show and Next is blocked', async () => {})
  test.skip('ONB-03 @onboarding @p1: vault step can be skipped — assert flow proceeds without a vault and no vault exists afterwards', async () => {})
  test.skip('ONB-04 @onboarding @p2: import step accepts a Netscape HTML file — assert imported bookmarks appear in the library', async () => {})
})
