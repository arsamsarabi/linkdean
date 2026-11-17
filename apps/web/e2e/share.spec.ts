import { test } from '@playwright/test'

test.describe('Share', () => {
  test.skip('SHARE-01 @share @p1: share folder with read access to another user — assert invitee sees an invite in notifications', async () => {})
  test.skip('SHARE-02 @share @p1: invitee accepts share and sees folder — assert folder appears under Shared and in the library', async () => {})
  test.skip('SHARE-03 @share @p1: read-only invitee cannot edit shared bookmarks — assert edit actions are hidden or disabled', async () => {})
  test.skip('SHARE-04 @share @p1: owner revokes share — assert folder disappears for the invitee', async () => {})
  test.skip('SHARE-05 @share @p1: enable public list link from Share drawer — assert /l/{slug} link is shown and copyable', async () => {})
  test.skip('SHARE-06 @share @p2: save a shared bookmark to my library — assert copy lands in Unsorted', async () => {})
  test.skip('SHARE-07 @share @p2: Shared page lists invites, shared with me and shared by me — assert all three groups render with counts', async () => {})
  test.skip('SHARE-08 @share @p1: invitee declines share — assert nothing appears in the invitee library', async () => {})
  test.skip('SHARE-09 @share @p2: like, rate and comment on a shared bookmark — assert counts update and owner is notified', async () => {})
})
