import { test } from '@playwright/test'

test.describe('Admin', () => {
  test.skip('ADM-01 @admin @p1: non-admin cannot open /admin — assert redirect or forbidden state is shown', async () => {})
  test.skip('ADM-02 @admin @p1: admin sees tabs General, Users, Reports, Bans, Invites, Quotas, Email, Screenshots, Search index — assert each ?tab= deep link shows its panel', async () => {})
  test.skip('ADM-03 @admin @p1: toggle open registration — assert /register reflects the setting', async () => {})
  test.skip('ADM-04 @admin @p1: create registration invite link — assert /register?invite= link is shown', async () => {})
  test.skip('ADM-05 @admin @p1: ban a user with reason and duration — assert user appears under Bans', async () => {})
  test.skip('ADM-06 @admin @p1: unban a user — assert user leaves Bans and can sign in', async () => {})
  test.skip('ADM-07 @admin @p2: abuse report queue Dismiss, Hide content, Warn and Ban — assert report resolves with the chosen action', async () => {})
  test.skip('ADM-08 @admin @p2: promote and demote admin protects last admin — assert last admin demote is refused', async () => {})
})
