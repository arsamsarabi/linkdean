import type { ReactElement } from 'react'
import { Alert, Stack } from '@mantine/core'
import { IconInfoCircle } from '@tabler/icons-react'
import { CreateVaultForm } from './CreateVaultForm'
import { GlobalPassphraseCard } from './GlobalPassphraseCard'
import { VaultList } from './VaultList'

export function VaultPanel(): ReactElement {
  return (
    <Stack gap="lg" maw={560}>
      <Alert icon={<IconInfoCircle size={16} />} color="violet" variant="light">
        Each vault has its own passphrase. Bookmark content is encrypted at rest and never reaches search until you
        unlock the vault.
      </Alert>
      <VaultList />
      <CreateVaultForm />
      <GlobalPassphraseCard />
    </Stack>
  )
}
