import type { ReactElement, ReactNode } from 'react'
import { Alert } from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { TwoColumnLayout } from '@/components/two-column-layout'
import { CreateVaultForm } from './CreateVaultForm'
import { GlobalPassphraseCard } from './GlobalPassphraseCard'
import { VaultList } from './VaultList'

/** Vaults on the left, account-wide vault settings on the right. */
export function VaultPanel({ settings }: { settings: ReactNode }): ReactElement {
  return (
    <TwoColumnLayout
      left={
        <>
          <Alert icon={<IconInfoCircle size={16} />} color="violet" variant="light">
            Each vault has its own passphrase. Bookmark content is encrypted at rest and never reaches search until you
            unlock the vault.
          </Alert>
          <VaultList />
          <CreateVaultForm />
        </>
      }
      right={
        <>
          <GlobalPassphraseCard />
          {settings}
        </>
      }
    />
  )
}
