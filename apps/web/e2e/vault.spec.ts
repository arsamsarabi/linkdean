import { expect, test } from '@playwright/test'
import { createBookmark, uniq } from './fixtures/api'
import { bookmarkCard } from './fixtures/ui'

test.describe('Vault', () => {
  // ponytail: the UI has no "Set up vault" prompt yet; with no vault the modal just lists "Out of vault".
  // Assert that, so a regression that offers a phantom vault (or moves silently) fails.
  test('VAULT-01 @vault @smoke: Move to vault with no vault offers no vault destination — assert only "Out of vault" is listed and Move needs a choice (no "Set up vault" prompt exists in the UI yet)', async ({
    page,
    request,
  }) => {
    const title = uniq('VAULT-01 bookmark')
    await createBookmark(request, { title })
    await page.goto('/library')
    await bookmarkCard(page, title).getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Move to vault' }).click()

    const modal = page.getByRole('dialog', { name: 'Move to vault' })
    await expect(modal.getByRole('button', { name: 'Move', exact: true })).toBeDisabled()
    await modal.getByRole('combobox', { name: 'Destination' }).click()
    const options = page.getByRole('option')
    await expect(options).toHaveText(['Out of vault'])
  })
  test.skip('VAULT-02 @vault @p1: create vault with passphrase shows recovery key once — assert recovery key is displayed and not shown again after close', async () => {})
  test.skip('VAULT-03 @vault @p1: unlock vault with passphrase opens Vaults page — assert vault items are listed', async () => {})
  test.skip('VAULT-04 @vault @p1: wrong vault passphrase shows error — assert vault stays locked', async () => {})
  test.skip('VAULT-05 @vault @p1: lock vault hides content — assert Vaults page requires unlock again', async () => {})
  test.skip('VAULT-06 @vault @p1: move bookmark into vault removes it from Library — assert card leaves the library and is under Vaults', async () => {})
  test.skip('VAULT-07 @vault @p1: move bookmark out of vault — assert bookmark returns to the library', async () => {})
  test.skip('VAULT-08 @vault @p2: vault move checklist lists blockers — assert blocking items are named in the dialog', async () => {})
  test.skip('VAULT-09 @vault @p2: recovery key can be replaced with passphrase — assert new key shown once', async () => {})
})
