import { randomUUID } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { expect, test } from '@playwright/test'
import { credentialsFile, E2E_PASSWORD } from './fixtures/api'

test.describe('Auth', () => {
  // Sign-in flows start signed out; the chromium project's storageState would redirect away from /login.
  test.use({ storageState: { cookies: [], origins: [] } })

  test('AUTH-01 @auth @smoke: register or sign-in lands past onboarding into app — assert URL is the app shell (/ or /library), not /login or /onboarding', async ({
    page,
  }) => {
    // The setup user (registered and onboarded by auth.setup.ts), signed in through the UI.
    const { email } = JSON.parse(readFileSync(credentialsFile, 'utf8')) as { email: string }

    await page.goto('/login')
    await page.getByLabel('Email').fill(email)
    await page.getByRole('textbox', { name: 'Password' }).fill(E2E_PASSWORD)
    await page.getByRole('button', { name: 'Sign in', exact: true }).click()

    await expect(page).toHaveURL(/\/(library)?$/)
    await expect(page.getByRole('link', { name: 'Library' }).first()).toBeVisible()
  })
  test('AUTH-02 @auth @smoke: wrong password shows error — assert inline error is visible and the user stays on /login', async ({
    page,
  }) => {
    // An unknown address fails the same way as a wrong password, so no registration is needed.
    await page.goto('/login')
    await page.getByLabel('Email').fill(`e2e-nobody-${randomUUID()}@example.test`)
    await page.getByRole('textbox', { name: 'Password' }).fill('definitely-not-the-password')
    await page.getByRole('button', { name: 'Sign in', exact: true }).click()

    // The inline Alert in the form (a toast with the same text also fires, so scope to the Alert).
    await expect(page.locator('.mantine-Alert-root')).toContainText(/invalid|incorrect|wrong|401|unauthor/i)
    await expect(page).toHaveURL(/\/login/)
  })
  test.skip('AUTH-03 @auth @p1: register with terms checkbox when TERMS_VERSION is set — assert submit is blocked until the checkbox is ticked', async () => {})
  test.skip('AUTH-04 @auth @p1: sign out returns to login — assert /login is shown and /library redirects back to /login', async () => {})
  test.skip('AUTH-05 @auth @p1: forgot password shows confirmation — assert neutral "check your email" message appears for any address', async () => {})
  test.skip('AUTH-06 @auth @p2: reset password with valid link then sign in — assert new password signs in and old password is rejected', async () => {})
  test.skip('AUTH-07 @auth @p2: verify email link marks account verified — assert verified state shown and unverified banner is gone', async () => {})
  test.skip('AUTH-08 @auth @p1: signed-out visit to a protected route redirects to login — assert /library redirects to /login', async () => {})
  test.skip('AUTH-09 @auth @p2: banned user is sent to /banned with reason — assert ban reason is visible and the app is not reachable', async () => {})
})
