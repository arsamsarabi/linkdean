import { test } from '@playwright/test'

test.describe('Vault', () => {
  test.skip('VAULT-01 @vault @smoke: Move to vault with no vault shows Set up vault — assert Set up vault prompt appears instead of a move', async () => {})
  test.skip('VAULT-02 @vault @p1: create vault with passphrase shows recovery key once — assert recovery key is displayed and not shown again after close', async () => {})
  test.skip('VAULT-03 @vault @p1: unlock vault with passphrase opens Vaults page — assert vault items are listed', async () => {})
  test.skip('VAULT-04 @vault @p1: wrong vault passphrase shows error — assert vault stays locked', async () => {})
  test.skip('VAULT-05 @vault @p1: lock vault hides content — assert Vaults page requires unlock again', async () => {})
  test.skip('VAULT-06 @vault @p1: move bookmark into vault removes it from Library — assert card leaves the library and is under Vaults', async () => {})
  test.skip('VAULT-07 @vault @p1: move bookmark out of vault — assert bookmark returns to the library', async () => {})
  test.skip('VAULT-08 @vault @p2: vault move checklist lists blockers — assert blocking items are named in the dialog', async () => {})
  test.skip('VAULT-09 @vault @p2: recovery key can be replaced with passphrase — assert new key shown once', async () => {})
})
