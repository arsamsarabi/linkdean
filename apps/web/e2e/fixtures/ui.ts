import type { Locator, Page } from '@playwright/test'

/**
 * The bookmark card (grid/list/search result) whose title link reads exactly `title`:
 * the nearest ancestor holding both the "Bookmark actions" button and a folder chip link.
 */
export function bookmarkCard(page: Page, title: string): Locator {
  return page
    .getByRole('link', { name: title, exact: true })
    .locator('xpath=ancestor::div[.//button[@aria-label="Bookmark actions"] and .//a[contains(@href,"category=")]][1]')
}
