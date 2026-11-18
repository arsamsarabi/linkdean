import { expect, test } from '@playwright/test'
import { clearUnsortedQueue, createBookmark, createCategory, uniq } from './fixtures/api'

test.describe('Filing queue', () => {
  test.describe.configure({ mode: 'serial' })

  test.beforeEach(async ({ request }) => {
    await clearUnsortedQueue(request)
  })

  test('FILE-01 @filing @p1: File one by one starts from Unsorted — assert first card is the newest Unsorted bookmark', async ({
    page,
    request,
  }) => {
    const title = uniq('FILE-01 newest')
    await createBookmark(request, { title })
    await page.goto('/file')
    await expect(page.getByLabel('Title')).toHaveValue(title)
  })

  test('FILE-02 @filing @p1: File requires a folder — assert File is disabled until a folder is chosen', async ({
    page,
    request,
  }) => {
    const title = uniq('FILE-02')
    await createBookmark(request, { title })
    await page.goto('/file')
    await expect(page.getByLabel('Title')).toHaveValue(title)
    await expect(page.getByRole('button', { name: 'File', exact: true })).toBeDisabled()
  })

  test('FILE-03 @filing @p1: File assigns folder and advances — assert next bookmark is shown and the first leaves Unsorted', async ({
    page,
    request,
  }) => {
    const folder = uniq('FILE-03 folder')
    const first = uniq('FILE-03 first')
    const second = uniq('FILE-03 second')
    await createCategory(request, folder)
    await createBookmark(request, { title: first })
    await createBookmark(request, { title: second })

    await page.goto('/file')
    await expect(page.getByLabel('Title')).toHaveValue(second)
    await page.getByRole('combobox', { name: 'Categories' }).fill(folder)
    await page.getByRole('option', { name: folder, exact: true }).click()
    await page.getByRole('button', { name: 'File', exact: true }).click()

    await expect(page.getByLabel('Title')).toHaveValue(first)
    await page.goto('/library')
    await page.getByRole('link', { name: folder }).first().click()
    await expect(page.getByText(second)).toBeVisible()
  })

  test('FILE-04 @filing @p1: Skip leaves bookmark in Unsorted — assert next card shown and bookmark remains', async ({
    page,
    request,
  }) => {
    const a = uniq('FILE-04 a')
    const b = uniq('FILE-04 b')
    await createBookmark(request, { title: a })
    await createBookmark(request, { title: b })

    await page.goto('/file')
    await expect(page.getByLabel('Title')).toHaveValue(b)
    await page.getByRole('button', { name: 'Skip' }).click()
    await expect(page.getByLabel('Title')).toHaveValue(a)

    await page.goto('/library')
    await expect(page.getByText(b)).toBeVisible()
  })

  test('FILE-05 @filing @p1: Delete moves bookmark to Deleted items — assert bookmark appears in Trash', async ({
    page,
    request,
  }) => {
    const title = uniq('FILE-05')
    await createBookmark(request, { title })
    await page.goto('/file')
    await expect(page.getByLabel('Title')).toHaveValue(title)
    await page.getByRole('button', { name: 'Delete' }).click()
    await expect(page.getByText(/All done|Nothing to file|Moved to Deleted/i).first()).toBeVisible()
    await page.goto('/trash')
    await expect(page.getByText(title)).toBeVisible()
  })

  test.skip('FILE-06 @filing @p2: keyboard shortcuts 1–5, Enter, S and D work — assert each key triggers its action', async () => {})
})
