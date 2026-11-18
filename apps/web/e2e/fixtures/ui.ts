import type { Locator, Page } from '@playwright/test'

/**
 * The bookmark card (grid/list/search result) whose title reads exactly `title`.
 * Title sits in a Group with "Bookmark actions"; climb to the Mantine Card so folder chips
 * (siblings below that Group) stay in scope.
 */
export function bookmarkCard(page: Page, title: string): Locator {
  return page
    .getByRole('link', { name: title, exact: true })
    .locator('xpath=ancestor::div[contains(@class,"mantine-Card-root")][1]')
}

/** The settings vault card (Paper) whose heading text is exactly `name`. */
export function vaultSettingsCard(page: Page, name: string): Locator {
  return page.getByText(name, { exact: true }).locator('xpath=ancestor::div[contains(@class,"mantine-Paper-root")][1]')
}
