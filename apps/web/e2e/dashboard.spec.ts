import { expect, test } from '@playwright/test'

test.describe('Dashboard', () => {
  test('DASH-01 @dashboard @smoke: home dashboard loads for signed-in user — assert greeting header and dashboard sections render at /', async ({
    page,
  }) => {
    await page.goto('/')
    await expect(page.getByText(/^Good (morning|afternoon|evening)/)).toBeVisible()
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible()
  })
  test.skip('DASH-02 @dashboard @p1: create a new dashboard from the switcher — assert new board is selected and listed', async () => {})
  test.skip('DASH-03 @dashboard @p1: Edit layout adds a Pinned section — assert section appears with the chosen title', async () => {})
  test.skip('DASH-04 @dashboard @p1: pin a bookmark from the + on a pinned section — assert bookmark tile appears in the section', async () => {})
  test.skip('DASH-05 @dashboard @p1: Filter this dashboard narrows sections as you type — assert non-matching bookmarks hide and Esc clears', async () => {})
  test.skip('DASH-06 @dashboard @p1: opening a bookmark records Most opened — assert open count increments', async () => {})
  test.skip('DASH-07 @dashboard @p2: resize and move a section in the editor — assert new size and order persist after reload', async () => {})
})
