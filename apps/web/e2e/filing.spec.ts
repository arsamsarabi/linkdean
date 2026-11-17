import { test } from '@playwright/test'

test.describe('Filing queue', () => {
  test.skip('FILE-01 @filing @p1: File one by one starts from Unsorted — assert first card is the newest Unsorted bookmark', async () => {})
  test.skip('FILE-02 @filing @p1: File requires a folder — assert File is disabled until a folder is chosen', async () => {})
  test.skip('FILE-03 @filing @p1: File assigns folder and advances — assert next bookmark is shown and the first leaves Unsorted', async () => {})
  test.skip('FILE-04 @filing @p1: Skip leaves bookmark in Unsorted — assert next card shown and bookmark remains', async () => {})
  test.skip('FILE-05 @filing @p1: Delete moves bookmark to Deleted items — assert bookmark appears in Trash', async () => {})
  test.skip('FILE-06 @filing @p2: keyboard shortcuts 1–5, Enter, S and D work — assert each key triggers its action', async () => {})
})
