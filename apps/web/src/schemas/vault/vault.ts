import { z } from 'zod'

export const vaultSchema = z.object({
  id: z.string(),
  name: z.string(),
  unlocked: z.boolean(),
  has_recovery_key: z.boolean(),
  in_global: z.boolean(),
  category_count: z.number().nullish(),
  bookmark_count: z.number().nullish(),
})

export const vaultsEnvelopeSchema = z.object({ vaults: z.array(vaultSchema) })

export const createdVaultSchema = z.object({ vault: vaultSchema, recovery_key: z.string() })

export const recoveryKeySchema = z.object({ recovery_key: z.string() })

export const globalPassphraseSchema = z.object({ enabled: z.boolean() })

export const vaultMoveBlockerSchema = z.object({
  kind: z.enum(['shared', 'public_link', 'other_category', 'dashboard']),
  subject_id: z.string(),
  related_id: z.string().nullish(),
  message: z.string(),
})

export type Vault = z.infer<typeof vaultSchema>
export type VaultMoveBlocker = z.infer<typeof vaultMoveBlockerSchema>
