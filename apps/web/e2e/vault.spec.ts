import { expect, test } from '@playwright/test'
import {
  createBookmark,
  createVault,
  E2E_VAULT_PASSPHRASE,
  lockAllVaults,
  lockVault,
  moveToVault,
  patchSettings,
  uniq,
} from './fixtures/api'
import { bookmarkCard, vaultSettingsCard } from './fixtures/ui'

test.describe('Vault', () => {
  test.describe.configure({ mode: 'serial' })

  // ponytail: the UI has no "Set up vault" prompt yet; with no vault the modal just lists "Out of vault".
  test('VAULT-01 @vault @smoke: Move to vault with no vault offers no vault destination — assert only "Out of vault" is listed and Move needs a choice (no "Set up vault" prompt exists in the UI yet)', async ({
    page,
  }) => {
    const title = uniq('VAULT-01 bookmark')
    // page.request shares the browser cookie jar (vault unlock is session-scoped).
    const request = page.request
    await lockAllVaults(request)
    await createBookmark(request, { title })
    await page.goto('/library')
    await expect(page.getByRole('link', { name: title, exact: true })).toBeVisible()
    await bookmarkCard(page, title).getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Move to vault' }).click()

    // Prefer Destination/Move over dialog name — Mantine title a11y can miss the name in CI.
    await expect(page.getByRole('combobox', { name: 'Destination' })).toBeVisible()
    await expect(page.getByRole('button', { name: 'Move', exact: true })).toBeDisabled()
    await page.getByRole('combobox', { name: 'Destination' }).click()
    await expect(page.getByRole('option')).toHaveText(['Out of vault'])
  })

  test('VAULT-02 @vault @p1: create vault with passphrase shows recovery key once — assert recovery key is displayed and not shown again after close', async ({
    page,
  }) => {
    const name = uniq('VAULT-02')
    await page.goto('/settings')
    await page.getByRole('tab', { name: 'Vault' }).click()
    const createBlock = page
      .locator('.mantine-Paper-root')
      .filter({ has: page.getByRole('heading', { name: 'New vault' }) })
    await createBlock.getByRole('textbox', { name: 'Name', exact: true }).fill(name)
    await createBlock.getByRole('textbox', { name: 'Passphrase', exact: true }).fill(E2E_VAULT_PASSPHRASE)
    await createBlock.getByRole('textbox', { name: 'Confirm passphrase', exact: true }).fill(E2E_VAULT_PASSPHRASE)
    await createBlock.getByRole('button', { name: 'Create vault' }).click()

    const recovery = page.getByRole('dialog', { name: 'Save your recovery key' })
    await expect(recovery).toBeVisible()
    // Mantine Code renders as a pre/code hybrid; assert the one-time key text is present.
    await expect(recovery.locator('.mantine-Code-root')).not.toBeEmpty()
    await recovery.getByLabel('I saved this recovery key').check()
    await recovery.getByRole('button', { name: 'Done' }).click()
    await expect(recovery).toBeHidden()
    await expect(page.getByText(name, { exact: true })).toBeVisible()
  })

  test('VAULT-03 @vault @p1: unlock vault with passphrase opens Vaults page — assert vault items are listed', async ({
    page,
    request,
  }) => {
    const name = uniq('VAULT-03')
    const title = uniq('VAULT-03 item')
    const { id } = await createVault(request, name)
    const bookmarkId = await createBookmark(request, { title })
    await moveToVault(request, [bookmarkId], id)
    await lockVault(request, id)
    await page.reload() // drop any stale unlocked client state

    await page.goto('/settings')
    await page.getByRole('tab', { name: 'Vault' }).click()
    const card = vaultSettingsCard(page, name)
    await card.getByRole('textbox', { name: `Passphrase for ${name}` }).fill(E2E_VAULT_PASSPHRASE)
    await card.getByRole('button', { name: 'Unlock' }).click()
    await expect(card.getByText('Unlocked')).toBeVisible()

    await page.goto('/vaults')
    await expect(page.getByText(title)).toBeVisible()
  })

  test('VAULT-04 @vault @p1: wrong vault passphrase shows error — assert vault stays locked', async ({
    page,
    request,
  }) => {
    const name = uniq('VAULT-04')
    const { id } = await createVault(request, name)
    await lockVault(request, id)

    await page.goto('/settings')
    await page.getByRole('tab', { name: 'Vault' }).click()
    const card = vaultSettingsCard(page, name)
    await card.getByRole('textbox', { name: `Passphrase for ${name}` }).fill('wrong-passphrase!!')
    await card.getByRole('button', { name: 'Unlock' }).click()
    await expect(page.getByText(/incorrect|invalid|wrong|failed|passphrase/i).first()).toBeVisible()
    await expect(card.getByText('Locked')).toBeVisible()
  })

  test('VAULT-05 @vault @p1: lock vault hides content — assert Vaults page requires unlock again', async ({
    page,
    request,
  }) => {
    const name = uniq('VAULT-05')
    const title = uniq('VAULT-05 item')
    const { id } = await createVault(request, name)
    const bookmarkId = await createBookmark(request, { title })
    await moveToVault(request, [bookmarkId], id)

    await page.goto('/settings')
    await page.getByRole('tab', { name: 'Vault' }).click()
    const card = vaultSettingsCard(page, name)
    await expect(card.getByText('Unlocked')).toBeVisible()
    await card.getByRole('button', { name: 'Lock', exact: true }).click()
    await expect(card.getByText('Locked')).toBeVisible()

    // Other vaults from earlier specs may still be open; lock everything so /vaults must bounce.
    await lockAllVaults(request)
    await page.goto('/vaults')
    await expect(page).toHaveURL(/\/library/)
    await expect(page.getByText(title)).toBeHidden()
  })

  test('VAULT-06 @vault @p1: move bookmark into vault removes it from Library — assert card leaves the library and is under Vaults', async ({
    page,
    request,
  }) => {
    const name = uniq('VAULT-06')
    const title = uniq('VAULT-06 bookmark')
    // Default show_vault_in_library keeps vault cards in Library; turn it off for this assertion.
    await patchSettings(request, { show_vault_in_library: false })
    await createVault(request, name)
    await createBookmark(request, { title })

    await page.goto('/library')
    await bookmarkCard(page, title).getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Move to vault' }).click()
    const modal = page.getByRole('dialog', { name: 'Move to vault' })
    await modal.getByRole('combobox', { name: 'Destination' }).click()
    await page.getByRole('option', { name, exact: true }).click()
    await modal.getByRole('button', { name: 'Move', exact: true }).click()
    await expect(modal).toBeHidden()
    await expect(page.getByText(title)).toBeHidden()

    await page.goto('/vaults')
    await expect(page.getByText(title)).toBeVisible()
  })

  test('VAULT-07 @vault @p1: move bookmark out of vault — assert bookmark returns to the library', async ({
    page,
    request,
  }) => {
    const name = uniq('VAULT-07')
    const title = uniq('VAULT-07 bookmark')
    const { id } = await createVault(request, name)
    const bookmarkId = await createBookmark(request, { title })
    await moveToVault(request, [bookmarkId], id)

    await page.goto('/vaults')
    await bookmarkCard(page, title).getByRole('button', { name: 'Bookmark actions' }).click()
    await page.getByRole('menuitem', { name: 'Move to vault' }).click()
    const modal = page.getByRole('dialog', { name: 'Move to vault' })
    await modal.getByRole('combobox', { name: 'Destination' }).click()
    await page.getByRole('option', { name: 'Out of vault' }).click()
    await modal.getByRole('button', { name: 'Move', exact: true }).click()
    await expect(modal).toBeHidden()
    await expect(page.getByText(title)).toBeHidden()

    await page.goto('/library')
    await expect(page.getByText(title)).toBeVisible()
  })

  test.skip('VAULT-08 @vault @p2: vault move checklist lists blockers — assert blocking items are named in the dialog', async () => {})
  test.skip('VAULT-09 @vault @p2: recovery key can be replaced with passphrase — assert new key shown once', async () => {})
})
