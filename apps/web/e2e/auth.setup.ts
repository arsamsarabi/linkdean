import { randomUUID } from 'node:crypto'
import { writeFileSync } from 'node:fs'
import { test as setup } from '@playwright/test'
import { completeOnboarding, credentialsFile, registerUser } from './fixtures/api'

export const authFile = 'e2e/.auth/user.json'

/**
 * Registers a fresh user on the throwaway API, finishes onboarding, and saves the browser
 * session (cookie `linkdean_session` + localStorage `linkdean_signed_in`) for the chromium project.
 */
setup('register user and save storageState', async ({ page }) => {
  const email = `e2e-${randomUUID()}@example.test`
  // The page's context.request shares its cookie jar, so the register cookie lands in storageState.
  const request = page.context().request

  await registerUser(request, { email })
  writeFileSync(credentialsFile, JSON.stringify({ email }))
  await completeOnboarding(request)

  await page.goto('/login')
  await page.evaluate(() => localStorage.setItem('linkdean_signed_in', '1'))
  await page.context().storageState({ path: authFile })
})
