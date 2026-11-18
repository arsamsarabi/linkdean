import type { Locator, Page } from '@playwright/test'

/**
 * The bookmark card (grid/list/search result) whose title reads exactly `title`.
 * Title is an external `<a>` (not a router link); scope to the nearest ancestor that holds
 * "Bookmark actions". Do not require a category= chip — Unsorted and vault cards still have the menu.
 */
export function bookmarkCard(page: Page, title: string): Locator {
  return page
    .getByRole('link', { name: title, exact: true })
    .locator('xpath=ancestor::*[.//button[@aria-label="Bookmark actions"]][1]')
}

/** The settings vault card (Paper) whose heading text is exactly `name`. */
export function vaultSettingsCard(page: Page, name: string): Locator {
  return page.getByText(name, { exact: true }).locator('xpath=ancestor::div[contains(@class,"mantine-Paper-root")][1]')
}
