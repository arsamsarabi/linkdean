import { expect, test } from '@playwright/test'
import { createBookmark, createCategory, setBookmarkCategories, uniq } from './fixtures/api'
import { bookmarkCard } from './fixtures/ui'

test.describe('Search', () => {
  test('SRCH-01 @search @smoke: search shows folder chip matching library (not stale Unsorted) — assert result chip names the bookmark’s current folder', async ({
    page,
    request,
  }) => {
    const title = uniq('SRCH01needle')
    const folder = uniq('SRCH-01 folder')
    const search = (): Promise<unknown[]> =>
      request
        .get(`/api/search?q=${encodeURIComponent(title)}`)
        .then((r) => r.json())
        .then((b: { hits: unknown[] }) => b.hits)

    // Index the bookmark while it is still in Unsorted, then file it: the chip must follow the move.
    const id = await createBookmark(request, { title })
    // Meilisearch indexes asynchronously: wait until the API search sees the bookmark.
    await expect.poll(async () => (await search()).length).toBeGreaterThan(0)
    await setBookmarkCategories(request, id, [await createCategory(request, folder)])

    await page.goto('/library')
    await page.getByPlaceholder('Search bookmarks…').fill(title)
    const card = bookmarkCard(page, title)
    await expect(card.getByRole('link', { name: folder })).toBeVisible()
    await expect(card.getByRole('link', { name: 'Unsorted' })).toBeHidden()
  })
  test.skip('SRCH-02 @search @p1: command palette opens with Mod+K — assert palette is visible and Esc closes it', async () => {})
  test.skip('SRCH-03 @search @p1: command palette adds a bookmark — assert Add bookmark opens the form', async () => {})
  test.skip('SRCH-04 @search @p1: command palette jumps to a category — assert library opens filtered to that category', async () => {})
  test.skip('SRCH-05 @search @p1: header search finds a bookmark by title — assert matching result is listed', async () => {})
  test.skip('SRCH-06 @search @p2: vault bookmarks never appear in search — assert locked or unlocked vault titles are not listed', async () => {})
  test.skip('SRCH-07 @search @p1: search with no results shows empty state — assert empty message is visible', async () => {})
  test.skip('SRCH-08 @search @p2: deleted bookmarks do not appear in search — assert trashed bookmark is not listed', async () => {})
})
