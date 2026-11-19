import { type ReactElement } from 'react'
import { Checkbox, Paper, Stack, Switch, Text, Title } from '@mantine/core'
import { useQuotaUsage } from '@/hooks'
import { formatBytes } from '@/lib'
import type { UserSettings } from '@/schemas'

type Props = {
  settings: UserSettings
  saving: boolean
  onPatch: (p: Partial<UserSettings>) => void
}

/** Settings → Archive: archive bookmarks as they are saved, the formats to keep, and the space used. */
export function ArchiveSettingsPanel({ settings, saving, onPatch }: Props): ReactElement {
  const usage = useQuotaUsage().data
  const limit = usage?.limits.max_archive_bytes
  return (
    <Paper p="xl" withBorder maw={640}>
      <Stack gap="md">
        <Stack gap={4}>
          <Title order={4}>Archive</Title>
          <Text size="sm" c="dimmed">
            Keep a copy of a page in case it changes or goes away. Archive one bookmark from its menu, several from the
            Library&apos;s select mode, or every new bookmark here.
          </Text>
        </Stack>
        <Switch
          label="Archive new bookmarks"
          description="Each bookmark you save or import is archived in the formats below."
          checked={settings.archive_new}
          disabled={saving}
          onChange={(e) => onPatch({ archive_new: e.currentTarget.checked })}
        />
        <Checkbox.Group
          label="Formats"
          description="Also preselected when you archive by hand."
          value={settings.archive_formats}
          onChange={(v) => {
            if (v.length > 0) onPatch({ archive_formats: v as UserSettings['archive_formats'] })
          }}
        >
          <Stack gap="xs" mt="xs">
            <Checkbox value="html" label="Web page (HTML)" disabled={saving} />
            <Checkbox value="pdf" label="PDF" disabled={saving} />
          </Stack>
        </Checkbox.Group>
        {usage ? (
          <Text size="sm" c="dimmed">
            Archived copies use {formatBytes(usage.archive_bytes)}
            {limit != null ? ` of ${formatBytes(limit)}` : ''}.
          </Text>
        ) : null}
      </Stack>
    </Paper>
  )
}
