import { expect, test } from '@playwright/test'
import { createBookmark, uniq } from './fixtures/api'
import { seedFinishedHtmlArchive } from './fixtures/archive'
import { bookmarkCard } from './fixtures/ui'

test.describe('Archive', () => {
  test('ARC-01 @archive @p1: archive HTML from menu queues with toast — assert Archive… dialog and ready notice', async ({
    page,
    request,
  }) => {
    const title = uniq('ARC-01')
    await createBookmark(request, { title, url: 'https://example.com/arc-01' })
    await page.goto('/library')
    await bookmarkCard(page, title).getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Archive…' }).click()

    const dialog = page.getByRole('dialog', { name: 'Archive' })
    await expect(dialog.getByText('Keep the page as')).toBeVisible()
    await expect(dialog.getByLabel('Web page (HTML)')).toBeChecked()
    // QA 1.2: both formats off must block submit.
    await dialog.getByLabel('Web page (HTML)').uncheck()
    await dialog.getByLabel('PDF').uncheck()
    await expect(dialog.getByRole('button', { name: 'Archive', exact: true })).toBeDisabled()
    await dialog.getByLabel('Web page (HTML)').check()
    await dialog.getByRole('button', { name: 'Archive', exact: true }).click()
    await expect(page.getByText('Archiving. You can keep working; the copy appears when it is ready.')).toBeVisible()
  })

  test('ARC-02 @archive @p1: open archive viewer for seeded HTML — assert iframe, Live page, Download, and empty state', async ({
    page,
    request,
  }) => {
    const title = uniq('ARC-02')
    const id = await createBookmark(request, { title, url: 'https://example.com/arc-02' })
    await seedFinishedHtmlArchive(request, id, { pageText: 'ARC02bodyphrase' })

    await page.goto('/library')
    const card = bookmarkCard(page, title)
    await expect(card.getByRole('link', { name: 'archived' })).toBeVisible()
    await card.getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Open archive' }).click()
    await expect(page).toHaveURL(new RegExp(`/archive/${id}`))
    await expect(page.locator(`iframe[title="Archived copy: ${title}"]`)).toBeVisible()
    await expect(page.getByRole('link', { name: 'Live page' })).toHaveAttribute('href', 'https://example.com/arc-02')
    await expect(page.getByRole('link', { name: 'Download' })).toBeVisible()

    const bare = await createBookmark(request, { title: uniq('ARC-02 bare') })
    await page.goto(`/archive/${bare}`)
    await expect(page.getByText('No archived copy')).toBeVisible()
    await expect(page.getByText('Archive this bookmark from its menu to keep a copy here.')).toBeVisible()
  })

  test('ARC-03 @archive @p2: delete archive keeps the bookmark — assert badge disappears and title remains', async ({
    page,
    request,
  }) => {
    const title = uniq('ARC-03')
    const id = await createBookmark(request, { title, url: 'https://example.com/arc-03' })
    await seedFinishedHtmlArchive(request, id)

    await page.goto('/library')
    const card = bookmarkCard(page, title)
    await expect(card.getByRole('link', { name: 'archived' })).toBeVisible()
    await card.getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Edit' }).click()

    const drawer = page.getByRole('dialog').filter({ hasText: 'Archive' })
    await drawer.getByRole('button', { name: 'Delete archive' }).click()
    const confirm = page.getByRole('dialog', { name: 'Delete archive?' })
    await expect(
      confirm.getByText('Every archived copy of this bookmark is deleted. The bookmark stays.')
    ).toBeVisible()
    await confirm.getByRole('button', { name: 'Delete archive' }).click()
    await expect(page.getByText('Archive deleted')).toBeVisible()

    await page.goto('/library')
    const after = bookmarkCard(page, title)
    await expect(after.getByRole('link', { name: title, exact: true })).toBeVisible()
    await expect(after.getByRole('link', { name: 'archived' })).toHaveCount(0)
  })

  test('ARC-04 @archive @p2: archive again warns it replaces the copy — assert confirm dialog shows the replace warning', async ({
    page,
    request,
  }) => {
    const title = uniq('ARC-04')
    const id = await createBookmark(request, { title, url: 'https://example.com/arc-04' })
    await seedFinishedHtmlArchive(request, id)

    await page.goto('/library')
    await bookmarkCard(page, title).getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Archive…' }).click()
    const dialog = page.getByRole('dialog', { name: 'Archive' })
    await expect(dialog.getByText(/This replaces the copy from /)).toBeVisible()
    await expect(dialog.getByText('If archiving fails, the current copy is kept.')).toBeVisible()
  })

  test('ARC-05 @archive @p1: library search matches seeded HTML page text — assert body hit and miss', async ({
    page,
    request,
  }) => {
    const title = uniq('ARC-05')
    const phrase = `ARC05needle${uniq('x').replace(/\s/g, '')}`
    const id = await createBookmark(request, { title, url: 'https://example.com/arc-05' })
    await seedFinishedHtmlArchive(request, id, { pageText: phrase })

    const search = (): Promise<unknown[]> =>
      request
        .get(`/api/search?q=${encodeURIComponent(phrase)}`)
        .then((r) => r.json())
        .then((b: { hits: unknown[] }) => b.hits)
    await expect.poll(async () => (await search()).length).toBeGreaterThan(0)

    await page.goto('/library')
    await page.getByPlaceholder('Search bookmarks…').fill(phrase)
    await expect(bookmarkCard(page, title)).toBeVisible()

    await page.getByPlaceholder('Search bookmarks…').fill(`zzznomatch${Date.now()}`)
    await expect(page.getByRole('link', { name: title, exact: true })).toHaveCount(0)
  })
})
