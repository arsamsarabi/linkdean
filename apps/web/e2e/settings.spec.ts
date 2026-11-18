import { test } from '@playwright/test'

test.describe('Settings', () => {
  test.skip('SET-01 @settings @p1: settings tabs open via ?tab= — assert each tab deep link shows its panel', async () => {})
  test.skip('SET-02 @settings @p1: create API key shows plaintext once — assert ldk_ key is visible once and listed masked after', async () => {})
  test.skip('SET-03 @settings @p1: export bookmarks as Netscape HTML and JSON — assert download starts for each format', async () => {})
  test.skip('SET-04 @settings @p1: import Netscape HTML adds bookmarks — assert imported count is shown and bookmarks appear', async () => {})
  test.skip('SET-05 @settings @p1: import duplicate review offers Skip, Replace, Merge, Import anyway — assert choices apply per row and for all', async () => {})
  test.skip('SET-06 @settings @p1: profile edit changes display name and bio — assert new name shows in header and profile', async () => {})
  test.skip('SET-07 @settings @p1: change password signs out other sessions — assert new password works and old fails', async () => {})
  test.skip('SET-08 @settings @p2: schedule account deletion with confirm and cancel it — assert grace banner shows and cancel clears it', async () => {})
  test.skip('SET-09 @settings @p1: revoke API key — assert key leaves the list', async () => {})
  test.skip('SET-10 @settings @p2: schedule account deletion with confirm and cancel it — assert grace banner shows and cancel clears it', async () => {})
})
