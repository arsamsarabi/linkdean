import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { Vault } from '@/schemas'
import { useUi } from '@/stores'

function invalidateVaultRelated(qc: ReturnType<typeof useQueryClient>): void {
  void qc.invalidateQueries({ queryKey: ['vaults'] })
  void qc.invalidateQueries({ queryKey: ['me'] })
  void qc.invalidateQueries({ queryKey: ['categories'] })
  void qc.invalidateQueries({ queryKey: ['bookmarks'] })
  void qc.invalidateQueries({ queryKey: ['search'] })
}

export function useLockAllVaults(): UseMutationResult<unknown, Error, void> {
  const qc = useQueryClient()
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  return useMutation({
    mutationFn: () => api.lockAllVaults(),
    onSuccess: () => {
      setVaultUnlocked(false)
      resetDecrypted(qc)
    },
  })
}

export function useVaults(enabled = true): UseQueryResult<{ vaults: Vault[] }> {
  return useQuery({ queryKey: ['vaults'], queryFn: api.vaults, enabled })
}

export function useGlobalPassphrase(enabled = true): UseQueryResult<{ enabled: boolean }> {
  return useQuery({ queryKey: ['vaults', 'global'], queryFn: api.globalPassphrase, enabled })
}

/** Drops decrypted data. Invalidating alone would leave it cached and visible on Back. */
function resetDecrypted(qc: ReturnType<typeof useQueryClient>): void {
  for (const queryKey of [['bookmarks'], ['categories'], ['search'], ['dashboard']]) {
    void qc.resetQueries({ queryKey })
  }
  void qc.invalidateQueries({ queryKey: ['vaults'] })
  void qc.invalidateQueries({ queryKey: ['me'] })
}

function useLockAction<T>(action: (input: T) => Promise<unknown>): UseMutationResult<unknown, Error, T> {
  const qc = useQueryClient()
  return useMutation({ mutationFn: action, onSuccess: () => resetDecrypted(qc) })
}

function useVaultAction<T>(action: (input: T) => Promise<unknown>): UseMutationResult<unknown, Error, T> {
  const qc = useQueryClient()
  return useMutation({ mutationFn: action, onSuccess: () => invalidateVaultRelated(qc) })
}

export function useCreateVault(): UseMutationResult<
  { vault: Vault; recovery_key: string },
  Error,
  { name: string; passphrase: string }
> {
  const qc = useQueryClient()
  return useMutation({ mutationFn: api.createVault, onSuccess: () => invalidateVaultRelated(qc) })
}

export function useNewRecoveryKey(): UseMutationResult<
  { recovery_key: string },
  Error,
  { id: string; passphrase: string }
> {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, passphrase }) => api.newRecoveryKey(id, passphrase),
    onSuccess: () => invalidateVaultRelated(qc),
  })
}

export const useRenameVault = (): UseMutationResult<unknown, Error, { id: string; name: string }> =>
  useVaultAction(({ id, name }) => api.renameVault(id, name))

export const useDeleteVault = (): UseMutationResult<unknown, Error, { id: string; name: string; password?: string }> =>
  useLockAction(({ id, ...body }) => api.deleteVault(id, body))

export const useUnlockVault = (): UseMutationResult<unknown, Error, { id: string; passphrase: string }> =>
  useVaultAction(({ id, passphrase }) => api.unlockVault(id, passphrase))

export const useLockVault = (): UseMutationResult<unknown, Error, string> => useLockAction(api.lockVault)

export const useRecoverVault = (): UseMutationResult<
  unknown,
  Error,
  { id: string; recovery_key: string; new_passphrase: string }
> => useVaultAction(({ id, ...body }) => api.recoverVault(id, body))

export const useEnableGlobalPassphrase = (): UseMutationResult<unknown, Error, string> =>
  useVaultAction(api.enableGlobalPassphrase)

export const useChangeGlobalPassphrase = (): UseMutationResult<
  unknown,
  Error,
  { current_passphrase: string; new_passphrase: string }
> => useVaultAction(api.changeGlobalPassphrase)

export const useDisableGlobalPassphrase = (): UseMutationResult<unknown, Error, string> =>
  useVaultAction(api.disableGlobalPassphrase)

export const useUnlockAllVaults = (): UseMutationResult<unknown, Error, string> => useVaultAction(api.unlockAllVaults)

export const useJoinGlobal = (): UseMutationResult<unknown, Error, string> => useVaultAction(api.joinGlobal)

export const useLeaveGlobal = (): UseMutationResult<unknown, Error, string> => useVaultAction(api.leaveGlobal)

export const useMoveToVault = (): UseMutationResult<
  unknown,
  Error,
  { category_ids: string[]; bookmark_ids: string[]; destination_vault_id: string | null }
> => useVaultAction(api.moveToVault)
