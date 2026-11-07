import { type ReactElement } from 'react'
import { Paper, Stack, Text, Title } from '@mantine/core'
import { QueryErrorAlert } from '@/components'
import { useQuotaUsage } from '@/hooks'
import { SettingsSkeleton } from './SettingsSkeleton'

function line(label: string, used: number, limit: number | null | undefined, unit = ''): string {
  if (limit == null) return `${label}: ${used}${unit} (unlimited)`
  return `${label}: ${used}${unit} / ${limit}${unit}`
}

/** Shows platform quota usage when any limit is configured. */
export function QuotaUsagePanel(): ReactElement | null {
  const usage = useQuotaUsage()

  if (usage.isLoading) return <SettingsSkeleton />
  if (usage.isError) {
    return <QueryErrorAlert error={usage.error} onRetry={() => void usage.refetch()} title="Usage failed" />
  }
  const data = usage.data
  if (!data) return null

  const { limits } = data
  const anyLimit =
    limits.max_bookmarks != null ||
    limits.max_stored_image_bytes != null ||
    limits.max_enrich_actions_per_day != null ||
    limits.max_import_bytes != null ||
    limits.max_upload_image_bytes != null
  if (!anyLimit) return null

  return (
    <Paper p="lg" withBorder>
      <Stack gap="sm">
        <Title order={4}>Usage</Title>
        <Text size="sm" c="dimmed">
          Limits set by the instance admin. Empty means unlimited.
        </Text>
        {limits.max_bookmarks != null ? (
          <Text size="sm">{line('Bookmarks', data.bookmarks, limits.max_bookmarks)}</Text>
        ) : null}
        {limits.max_stored_image_bytes != null ? (
          <Text size="sm">{line('Stored images', data.stored_image_bytes, limits.max_stored_image_bytes, ' B')}</Text>
        ) : null}
        {limits.max_enrich_actions_per_day != null ? (
          <Text size="sm">
            {line(
              `Enrichment today (${data.enrich_period_start} UTC)`,
              data.enrich_actions_today,
              limits.max_enrich_actions_per_day
            )}
          </Text>
        ) : null}
        {limits.max_import_bytes != null ? (
          <Text size="sm" c="dimmed">
            Import file max: {limits.max_import_bytes} B
          </Text>
        ) : null}
        {limits.max_upload_image_bytes != null ? (
          <Text size="sm" c="dimmed">
            Upload image max: {limits.max_upload_image_bytes} B
          </Text>
        ) : null}
      </Stack>
    </Paper>
  )
}
