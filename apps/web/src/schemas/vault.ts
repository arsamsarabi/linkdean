import { z } from 'zod'

export const vaultStatusSchema = z.object({
  configured: z.boolean(),
  unlocked: z.boolean(),
})

export type VaultStatus = z.infer<typeof vaultStatusSchema>
