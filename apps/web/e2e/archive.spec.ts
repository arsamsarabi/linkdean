import { test } from '@playwright/test'

test.describe('Archive', () => {
  test.skip('ARC-01 @archive @p1: archive a bookmark as HTML from its menu — assert archived badge appears when done', async () => {})
  test.skip('ARC-02 @archive @p1: open archive viewer at /archive/:id — assert copy renders with format switch and download', async () => {})
  test.skip('ARC-03 @archive @p2: delete archive keeps the bookmark — assert badge disappears and bookmark remains', async () => {})
  test.skip('ARC-04 @archive @p2: archive again warns it replaces the copy — assert confirm dialog is shown', async () => {})
})
