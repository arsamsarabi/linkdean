import { expect, test } from '@playwright/test'
import { createBookmark, deleteBookmark, uniq } from './fixtures/api'

test.describe('Trash and library health', () => {
  test('TRASH-01 @trash @smoke: deleted bookmark can be restored from Deleted items — assert bookmark returns to the library', async ({
    page,
    request,
  }) => {
    const title = uniq('TRASH-01 bookmark')
    const id = await createBookmark(request, { title })
    await deleteBookmark(request, id)

    await page.goto('/trash')
    const row = page.getByRole('row').filter({ hasText: title })
    await row.getByRole('button', { name: 'Restore' }).click()
    await expect(row).toBeHidden()

    await page.goto('/library')
    await expect(page.getByText(title)).toBeVisible()
  })
  test.skip('TRASH-02 @trash @p1: purge from trash removes permanently — assert item is gone and cannot be restored', async () => {})
  test.skip('TRASH-03 @trash @p1: duplicates page lists duplicate groups — assert groups render with merge action', async () => {})
  test.skip('TRASH-04 @trash @p1: merge duplicates keeps one bookmark — assert single bookmark remains', async () => {})
  test.skip('TRASH-05 @trash @p1: broken links page lists failed links — assert open, recheck and remove actions are visible', async () => {})
  test.skip('TRASH-06 @trash @p2: remove broken bookmark moves it to Deleted items — assert bookmark appears in Trash', async () => {})
  test.skip('TRASH-07 @trash @p2: history page lists create, edit and delete events — assert events render in order', async () => {})
})
