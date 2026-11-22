import { useState, type ReactElement } from 'react'
import { Button, Group, Paper, Stack, Text, Title } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { api } from '@/api'
import { QueryErrorAlert } from '@/components'
import { useBillingEnabled, useQuotaUsage } from '@/hooks'
import { SettingsSkeleton } from './SettingsSkeleton'

function line(label: string, used: number, limit: number | null | undefined, unit = ''): string {
  if (limit == null) return `${label}: ${used}${unit} (unlimited)`
  return `${label}: ${used}${unit} / ${limit}${unit}`
}

function planLabel(plan: 'free' | 'pro' | null | undefined): string {
  if (plan === 'pro') return 'Pro'
  if (plan === 'free') return 'Free'
  return ''
}

/** Shows plan and quota usage when a managed plan or any limit is configured. */
export function QuotaUsagePanel(): ReactElement | null {
  const usage = useQuotaUsage()
  const billingEnabled = useBillingEnabled()
  const [busy, setBusy] = useState<'checkout' | 'portal' | null>(null)

  if (usage.isLoading) return <SettingsSkeleton />
  if (usage.isError) {
    return <QueryErrorAlert error={usage.error} onRetry={() => void usage.refetch()} title="Usage failed" />
  }
  const data = usage.data
  if (!data) return null

  const { limits, plan } = data
  const anyLimit =
    limits.max_bookmarks != null ||
    limits.max_stored_image_bytes != null ||
    limits.max_enrich_actions_per_day != null ||
    limits.max_import_bytes != null ||
    limits.max_upload_image_bytes != null ||
    limits.max_archive_bytes != null ||
    limits.max_vaults != null
  if (!plan && !anyLimit) return null

  async function openBilling(kind: 'checkout' | 'portal') {
    setBusy(kind)
    try {
      const { url } = kind === 'checkout' ? await api.billingCheckout() : await api.billingPortal()
      window.location.assign(url)
    } catch (e) {
      notifications.show({
        color: 'red',
        title: kind === 'checkout' ? 'Upgrade failed' : 'Billing portal failed',
        message: e instanceof Error ? e.message : 'Try again later.',
      })
      setBusy(null)
    }
  }

  return (
    <Paper p="lg" withBorder>
      <Stack gap="sm">
        <Title order={4}>Usage</Title>
        {plan ? (
          <>
            <Text size="sm" fw={500}>
              Plan: {planLabel(plan)}
            </Text>
            <Text size="sm" c="dimmed">
              Caps from your plan, tightened further if the instance admin set a lower limit.
            </Text>
          </>
        ) : (
          <Text size="sm" c="dimmed">
            Limits set by the instance admin. Empty means unlimited.
          </Text>
        )}
        {billingEnabled && plan === 'free' ? (
          <Group>
            <Button loading={busy === 'checkout'} onClick={() => void openBilling('checkout')}>
              Upgrade to Pro
            </Button>
          </Group>
        ) : null}
        {billingEnabled && (plan === 'pro' || data.has_billing_customer) ? (
          <Group>
            <Button variant="default" loading={busy === 'portal'} onClick={() => void openBilling('portal')}>
              Manage billing
            </Button>
          </Group>
        ) : null}
        {plan || limits.max_bookmarks != null ? (
          <Text size="sm">{line('Bookmarks', data.bookmarks, limits.max_bookmarks)}</Text>
        ) : null}
        {plan || limits.max_stored_image_bytes != null ? (
          <Text size="sm">{line('Stored images', data.stored_image_bytes, limits.max_stored_image_bytes, ' B')}</Text>
        ) : null}
        {plan || limits.max_archive_bytes != null ? (
          <Text size="sm">{line('Archived copies', data.archive_bytes, limits.max_archive_bytes, ' B')}</Text>
        ) : null}
        {plan || limits.max_vaults != null ? (
          <Text size="sm">{line('Vaults', data.vaults, limits.max_vaults)}</Text>
        ) : null}
        {plan || limits.max_enrich_actions_per_day != null ? (
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
