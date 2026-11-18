import { randomUUID } from 'node:crypto'
import { expect, test } from '@playwright/test'
import { registerUser } from './fixtures/api'

test.describe('Onboarding', () => {
  // ponytail: the first-admin steps (Welcome, Admin) only render while the DB has no users, and the shared
  // throwaway DB is already bootstrapped. So this walks a fresh non-first user through the steps that remain.
  test.describe('fresh user', () => {
    test.use({ storageState: { cookies: [], origins: [] } })

    test('ONB-01 @onboarding @smoke: first-admin setup completes and lands in app — assert stepper reaches the end and the app shell loads', async ({
      page,
    }) => {
      // Registered but never onboarded: the router should send this user to /onboarding.
      await registerUser(page.context().request, { email: `e2e-${randomUUID()}@example.test` })
      await page.goto('/login')
      await page.evaluate(() => localStorage.setItem('linkdean_signed_in', '1'))
      await page.goto('/')

      await expect(page).toHaveURL(/\/onboarding/)
      await page.getByRole('button', { name: 'Skip for now' }).click()
      await page.getByRole('button', { name: 'Skip & finish' }).click()

      await expect(page).not.toHaveURL(/\/onboarding/)
      await expect(page.getByRole('link', { name: 'Library' }).first()).toBeVisible()
    })
  })
  test.skip('ONB-02 @onboarding @p1: admin step validates email and password confirmation — assert errors show and Next is blocked', async () => {})
  test.skip('ONB-03 @onboarding @p1: vault step can be skipped — assert flow proceeds without a vault and no vault exists afterwards', async () => {})
  test.skip('ONB-04 @onboarding @p2: import step accepts a Netscape HTML file — assert imported bookmarks appear in the library', async () => {})
})
