import { test } from '@playwright/test'

test.describe('Auth', () => {
  test.skip('AUTH-01 @auth @smoke: register or sign-in lands past onboarding into app — assert URL is the app shell (/ or /library), not /login or /onboarding', async () => {})
  test.skip('AUTH-02 @auth @smoke: wrong password shows error — assert inline error is visible and the user stays on /login', async () => {})
  test.skip('AUTH-03 @auth @p1: register with terms checkbox when TERMS_VERSION is set — assert submit is blocked until the checkbox is ticked', async () => {})
  test.skip('AUTH-04 @auth @p1: sign out returns to login — assert /login is shown and /library redirects back to /login', async () => {})
  test.skip('AUTH-05 @auth @p1: forgot password shows confirmation — assert neutral "check your email" message appears for any address', async () => {})
  test.skip('AUTH-06 @auth @p2: reset password with valid link then sign in — assert new password signs in and old password is rejected', async () => {})
  test.skip('AUTH-07 @auth @p2: verify email link marks account verified — assert verified state shown and unverified banner is gone', async () => {})
  test.skip('AUTH-08 @auth @p1: signed-out visit to a protected route redirects to login — assert /library redirects to /login', async () => {})
  test.skip('AUTH-09 @auth @p2: banned user is sent to /banned with reason — assert ban reason is visible and the app is not reachable', async () => {})
})
