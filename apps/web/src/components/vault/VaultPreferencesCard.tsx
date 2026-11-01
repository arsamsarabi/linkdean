import type { ReactElement } from 'react'
import { Paper, Select, Stack, Switch, Title } from '@mantine/core'
import type { UserSettings } from '@/schemas'

type Props = {
  settings: UserSettings
  onPatch: (patch: Partial<UserSettings>) => void
}

export function VaultPreferencesCard({ settings, onPatch }: Props): ReactElement {
  return (
    <Paper p="lg" withBorder>
      <Stack>
        <Title order={4}>Vault items</Title>
        <Switch
          label="Show vault items in Library"
          description="When off, open vaults appear only on the Vaults page."
          checked={settings.show_vault_in_library}
          onChange={(e) => onPatch({ show_vault_in_library: e.currentTarget.checked })}
        />
        <Select
          label="When a vault item is deleted"
          description="Applies to bookmarks and categories deleted from now on."
          allowDeselect={false}
          value={settings.vault_delete_mode}
          onChange={(v) => v && onPatch({ vault_delete_mode: v as UserSettings['vault_delete_mode'] })}
          data={[
            { value: 'trash', label: 'Move to Trash' },
            { value: 'permanent', label: 'Remove immediately' },
          ]}
        />
      </Stack>
    </Paper>
  )
}
