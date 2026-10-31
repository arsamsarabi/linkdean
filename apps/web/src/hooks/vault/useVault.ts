import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseMutationResult,
  type UseQueryResult,
} from '@tanstack/react-query'
import { api } from '@/api'
import type { VaultStatus } from '@/schemas'
import { useUi } from '@/store'

function invalidateVaultRelated(qc: ReturnType<typeof useQueryClient>): void {
  void qc.invalidateQueries({ queryKey: ['vault'] })
  void qc.invalidateQueries({ queryKey: ['me'] })
  void qc.invalidateQueries({ queryKey: ['categories'] })
  void qc.invalidateQueries({ queryKey: ['bookmarks'] })
  void qc.invalidateQueries({ queryKey: ['search'] })
}

export function useVaultStatus(): UseQueryResult<VaultStatus> {
  return useQuery({
    queryKey: ['vault'],
    queryFn: api.vaultStatus,
  })
}

export function useVaultSetup(): UseMutationResult<unknown, Error, string> {
  const qc = useQueryClient()
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  return useMutation({
    mutationFn: (passphrase: string) => api.vaultSetup(passphrase),
    onSuccess: () => {
      setVaultUnlocked(true)
      invalidateVaultRelated(qc)
    },
  })
}

export function useVaultUnlock(): UseMutationResult<unknown, Error, string> {
  const qc = useQueryClient()
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  return useMutation({
    mutationFn: (passphrase: string) => api.vaultUnlock(passphrase),
    onSuccess: () => {
      setVaultUnlocked(true)
      invalidateVaultRelated(qc)
    },
  })
}

export function useVaultLock(): UseMutationResult<unknown, Error, void> {
  const qc = useQueryClient()
  const setVaultUnlocked = useUi((s) => s.setVaultUnlocked)
  return useMutation({
    mutationFn: () => api.vaultLock(),
    onSuccess: () => {
      setVaultUnlocked(false)
      invalidateVaultRelated(qc)
    },
  })
}
