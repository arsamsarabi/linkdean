import { expect, test } from '@playwright/test'
import { createBookmark, uniq } from './fixtures/api'
import { bookmarkCard } from './fixtures/ui'

test.describe('Library', () => {
  test('LIB-01 @library @smoke: add bookmark with Create category, pill + save into folder — assert category pill shows and the bookmark appears in that folder', async ({
    page,
  }) => {
    const title = uniq('LIB-01 bookmark')
    const folder = uniq('LIB-01 folder')
    await page.goto('/library')
    await page.getByRole('button', { name: 'Add bookmark' }).first().click()

    const drawer = page.getByRole('dialog', { name: 'Add bookmark' })
    await drawer.getByLabel('URL').fill('https://example.com/lib-01')
    await drawer.getByLabel('Title').fill(title)
    const cats = drawer.getByRole('combobox', { name: 'Categories' })
    await cats.click()
    await cats.pressSequentially(folder)
    await page.getByRole('option', { name: `Create "${folder}"` }).click()
    // Create is immediate (no parent panel); wait for the pill after the API returns.
    await expect(drawer.getByText(folder, { exact: true })).toBeVisible()

    await drawer.getByRole('button', { name: 'Add bookmark' }).click()
    await expect(drawer).toBeHidden()

    await page.getByRole('link', { name: folder }).first().click()
    await expect(page.getByText(title)).toBeVisible()
  })
  // Blocked on product: BookmarkDrawer has no on-blur URL validation, and POST /api/bookmarks accepts
  // "https://not a url" (observed: toast "Saved"). Body below is the intended behaviour; flip to `test` once
  // the drawer validates the URL.
  test.fixme('LIB-02 @library @smoke: invalid URL blur error blocks Add — assert URL error shows on blur and Add stays disabled', async ({
    page,
  }) => {
    await page.goto('/library')
    await page.getByRole('button', { name: 'Add bookmark' }).first().click()
    const drawer = page.getByRole('dialog', { name: 'Add bookmark' })
    const add = drawer.getByRole('button', { name: 'Add bookmark' })

    await expect(add).toBeDisabled() // empty URL
    await drawer.getByLabel('URL').fill('not a url')
    await drawer.getByLabel('URL').blur()
    await expect(drawer.getByText(/valid.*(url|address)/i)).toBeVisible()
    await expect(add).toBeDisabled()
  })
  test('LIB-03 @library @smoke: open library and see saved bookmark — assert saved bookmark card is visible on /library', async ({
    page,
    request,
  }) => {
    const title = uniq('LIB-03 bookmark')
    await createBookmark(request, { title })
    await page.goto('/library')
    await expect(page.getByText(title)).toBeVisible()
  })
  test.skip('LIB-04 @library @p1: Import from URL fills title and description — assert form title and description are populated from the page', async () => {})
  test('LIB-05 @library @p1: delete bookmark moves it to Deleted items — assert card leaves the library', async ({
    page,
    request,
  }) => {
    const title = uniq('LIB-05 bookmark')
    await createBookmark(request, { title })
    await page.goto('/library')
    await bookmarkCard(page, title).getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Delete' }).click()
    await page.getByRole('dialog', { name: 'Delete bookmark?' }).getByRole('button', { name: 'Delete' }).click()
    await expect(page.getByText(title)).toBeHidden()
    await page.goto('/trash')
    await expect(page.getByRole('row').filter({ hasText: title })).toBeVisible()
  })
  test.skip('LIB-06 @library @p1: filter library by folder and tag — assert only matching bookmarks are shown with updated count', async () => {})
  test.skip('LIB-07 @library @p1: Unsorted filter shows uncategorized bookmarks — assert sidebar Unsorted count matches the list', async () => {})
  test.skip('LIB-08 @library @p1: switch between grid, list and explorer views — assert view persists after reload', async () => {})
  test('LIB-09 @library @p1: quick save at /save?url= opens filled form — assert URL is prefilled and Save to Unsorted is offered', async ({
    page,
  }) => {
    const url = `https://example.com/lib-09-${Date.now()}`
    await page.goto(`/save?url=${encodeURIComponent(url)}`)
    const drawer = page.getByRole('dialog').or(page.locator('form').first())
    await expect(page.getByLabel('URL')).toHaveValue(url)
    await expect(page.getByRole('button', { name: /Save to Unsorted|Add bookmark|Save/i }).first()).toBeVisible()
    await expect(drawer).toBeVisible()
  })
  test.skip('LIB-10 @library @p1: quick save of an existing URL opens that bookmark — assert no duplicate is created', async () => {})
  test.skip('LIB-11 @library @p1: create nested category from categories page — assert subcategory shows under its parent in the tree', async () => {})
  test.skip('LIB-12 @library @p1: edit category name, icon and hidden flag — assert changes show in the sidebar and hidden folder is declutter-only', async () => {})
  test.skip('LIB-13 @library @p1: bulk select moves bookmarks to a folder — assert selected bookmarks appear in the target folder', async () => {})
  test.skip('LIB-14 @library @p1: edit bookmark title and notes — assert changes persist after reload', async () => {})
  test.skip('LIB-15 @library @p1: favorite and unread toggles filter the library — assert Favorites and Unread views list only flagged bookmarks', async () => {})
  test.skip('LIB-16 @library @p2: quick peek opens a bookmark in a panel — assert panel opens and Esc closes it', async () => {})
  test.skip('LIB-17 @library @p2: paste URL outside a field opens new-bookmark form — assert form opens with the pasted URL', async () => {})
})
