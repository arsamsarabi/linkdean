import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Vault } from '@/schemas'
import { VaultList } from './VaultList'

const unlock = vi.hoisted(() => vi.fn())
const lock = vi.hoisted(() => vi.fn())
const join = vi.hoisted(() => vi.fn())
const leave = vi.hoisted(() => vi.fn())
const newKey = vi.hoisted(() => vi.fn())
const data = vi.hoisted(() => ({ vaults: [] as Vault[], global: false }))
const idle = { mutateAsync: vi.fn(), isPending: false }

vi.mock('@/hooks', () => ({
  useVaults: () => ({ data: { vaults: data.vaults }, isLoading: false, isError: false }),
  useUnlockVault: () => ({ mutateAsync: unlock, isPending: false }),
  useLockVault: () => ({ mutateAsync: lock, isPending: false }),
  useRenameVault: () => idle,
  useNewRecoveryKey: () => ({ mutateAsync: newKey, isPending: false }),
  useJoinGlobal: () => ({ mutateAsync: join, isPending: false }),
  useLeaveGlobal: () => ({ mutateAsync: leave, isPending: false }),
  useGlobalPassphrase: () => ({ data: { enabled: data.global } }),
}))

vi.mock('./RecoverVaultModal', () => ({ RecoverVaultModal: () => null }))
vi.mock('./DeleteVaultModal', () => ({ DeleteVaultModal: () => null }))
vi.mock('./RecoveryKeyModal', () => ({ RecoveryKeyModal: () => null }))

const base: Vault = { id: 'v1', name: 'Work', unlocked: false, has_recovery_key: true, in_global: false }

describe('VaultList', () => {
  afterEach(cleanup)
  beforeEach(() => {
    data.global = false
    join.mockReset()
    leave.mockReset()
  })

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

  it('toggles global unlock without asking for a passphrase', async () => {
    data.global = true
    data.vaults = [{ ...base, unlocked: true }]
    render(
      <MantineProvider>
        <VaultList />
      </MantineProvider>
    )
    expect(screen.queryByLabelText(/global passphrase$/i, { selector: 'input[type=password]' })).toBeNull()
    await userEvent.click(screen.getByLabelText('Opens with the global passphrase'))
    expect(join).toHaveBeenCalledWith('v1')
  })

  it('leaves global unlock from the toggle', async () => {
    data.global = true
    data.vaults = [{ ...base, in_global: true }]
    render(
      <MantineProvider>
        <VaultList />
      </MantineProvider>
    )
    await userEvent.click(screen.getByLabelText('Opens with the global passphrase'))
    expect(leave).toHaveBeenCalledWith('v1')
  })

  it('needs a locked vault unlocked before it can join', () => {
    data.global = true
    data.vaults = [base]
    render(
      <MantineProvider>
        <VaultList />
      </MantineProvider>
    )
    expect(screen.getByRole('switch')).toBeDisabled()
    expect(screen.getByText('Unlock this vault to turn this on.')).toBeTruthy()
  })

  it('asks before replacing a recovery key', async () => {
    newKey.mockReset().mockResolvedValue({ recovery_key: 'new-key' })
    data.vaults = [{ ...base, unlocked: true }]
    render(
      <MantineProvider>
        <VaultList />
      </MantineProvider>
    )
    await userEvent.click(screen.getByRole('button', { name: 'Replace recovery key' }))
    expect(newKey).not.toHaveBeenCalled()
    await userEvent.click(await screen.findByRole('button', { name: 'Replace key' }))
    expect(newKey).toHaveBeenCalledWith('v1')
  })
})
