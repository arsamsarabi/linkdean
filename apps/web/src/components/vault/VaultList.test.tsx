import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Vault } from '@/schemas'
import { VaultList } from './VaultList'

const unlock = vi.hoisted(() => vi.fn())
const lock = vi.hoisted(() => vi.fn())
const data = vi.hoisted(() => ({ vaults: [] as Vault[] }))
const idle = { mutateAsync: vi.fn(), isPending: false }

vi.mock('@/hooks', () => ({
  useVaults: () => ({ data: { vaults: data.vaults }, isLoading: false, isError: false }),
  useUnlockVault: () => ({ mutateAsync: unlock, isPending: false }),
  useLockVault: () => ({ mutateAsync: lock, isPending: false }),
  useRenameVault: () => idle,
  useNewRecoveryKey: () => idle,
  useJoinGlobal: () => idle,
  useLeaveGlobal: () => idle,
  useGlobalPassphrase: () => ({ data: { enabled: false } }),
}))

vi.mock('./RecoverVaultModal', () => ({ RecoverVaultModal: () => null }))
vi.mock('./DeleteVaultModal', () => ({ DeleteVaultModal: () => null }))

const base: Vault = { id: 'v1', name: 'Work', unlocked: false, has_recovery_key: true, in_global: false }

describe('VaultList', () => {
  afterEach(cleanup)

  it('unlocks a vault with its passphrase', async () => {
    data.vaults = [base]
    render(
      <MantineProvider>
        <VaultList />
      </MantineProvider>
    )
    await userEvent.type(screen.getByLabelText('Passphrase for Work'), 'work-passphrase')
    await userEvent.click(screen.getByRole('button', { name: 'Unlock' }))
    expect(unlock).toHaveBeenCalledWith({ id: 'v1', passphrase: 'work-passphrase' })
  })

  it('locks an unlocked vault and shows its counts', async () => {
    data.vaults = [{ ...base, unlocked: true, category_count: 2, bookmark_count: 5 }]
    render(
      <MantineProvider>
        <VaultList />
      </MantineProvider>
    )
    expect(screen.getByText('2 categories, 5 bookmarks')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Lock' }))
    expect(lock).toHaveBeenCalledWith('v1')
  })
})
