import { test } from '@playwright/test'

test.describe('Public pages', () => {
  test.skip('PUB-01 @public @p1: public list /l/:slug loads without sign-in — assert list title and bookmarks are visible', async () => {})
  test.skip('PUB-02 @public @p1: revoked or rotated public link shows not found — assert 404 state is visible', async () => {})
  test.skip('PUB-03 @public @p1: public list hides notes and vault content — assert notes are absent', async () => {})
  test.skip('PUB-04 @public @p2: signed-in user sees Report on public list — assert report control opens dialog', async () => {})
})
