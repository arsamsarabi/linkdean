import { useState, type ReactElement } from 'react'
import { Button, Group, NumberInput, Paper, Stack, Text } from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCameraRotate } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components'
import { useRetryScreenshots, useScreenshotRetryStatus, useUpdatePlatformSettings } from '@/hooks'
import type { PlatformSettings, ScreenshotRetryStatus } from '@/schemas'

/** A whole number in a range, saved on blur. An empty or out-of-range entry snaps back. */
function RangeInput({
  label,
  description,
  value,
  min,
  max,
  suffix,
  disabled,
  onSave,
}: {
  label: string
  description: string
  value: number
  min: number
  max: number
  suffix: string
  disabled: boolean
  onSave: (v: number) => void
}): ReactElement {
  const [draft, setDraft] = useState<string | number>(value)
  return (
    <NumberInput
      key={value}
      label={label}
      description={description}
      min={min}
      max={max}
      allowDecimal={false}
      suffix={suffix}
      value={draft}
      disabled={disabled}
      onChange={setDraft}
      onBlur={() => {
        const n = Number(draft)
        if (draft !== '' && Number.isInteger(n) && n >= min && n <= max) {
          if (n !== value) onSave(n)
        } else {
          setDraft(value)
        }
      }}
    />
  )
}

const when = (iso: string): string =>
  new Date(iso).toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })

/** Why the button cannot be used now, or null when it can. */
function blockedReason(status: ScreenshotRetryStatus): string | null {
  if (!status.browser_configured) return 'Screenshots are off on this server (BROWSER_WS_URL is not set).'
  if (status.running) return 'Taking screenshots now. This updates when they finish.'
  if (status.next_available_at) return `Available again ${when(status.next_available_at)}.`
  if (status.failed === 0) return 'Nothing to retry.'
  return null
}

export function AdminScreenshotRetryPanel({ settings }: { settings: PlatformSettings }): ReactElement {
  const status = useScreenshotRetryStatus()
  const retry = useRetryScreenshots()
  const save = useUpdatePlatformSettings()

  function saveSetting(body: { screenshot_retry_cooldown_hours?: number; screenshot_auto_retry_days?: number }): void {
    save.mutate(body, {
      onSuccess: () => notifications.show({ message: 'Platform settings saved' }),
      onError: (err) => notifications.show({ color: 'red', message: String(err) }),
    })
  }

  const data = status.data
  const blocked = data ? blockedReason(data) : null
  const failed = data?.failed ?? 0

  return (
    <Paper p="xl" withBorder>
      <Stack gap="md">
        <Text fw={500}>Screenshots</Text>
        <Text size="sm" c="dimmed">
          A screenshot that fails, often after a large import, leaves the bookmark on its Open Graph image or favicon.
          Failed ones are retried automatically, and you can retry them sooner here.
        </Text>
        {status.isError ? (
          <QueryErrorAlert
            error={status.error}
            onRetry={() => void status.refetch()}
            title="Screenshot status failed"
          />
        ) : null}
        <Group justify="space-between" align="center" gap="sm">
          <Stack gap={2}>
            <Text size="sm">
              {data
                ? `${failed.toLocaleString()} failed screenshot${failed === 1 ? '' : 's'} can be retried`
                : 'Checking failed screenshots…'}
            </Text>
            {blocked ? (
              <Text size="xs" c="dimmed">
                {blocked}
              </Text>
            ) : null}
          </Stack>
          <Button
            leftSection={<IconCameraRotate size={16} stroke={1.5} />}
            loading={retry.isPending}
            disabled={!data || blocked !== null}
            onClick={() =>
              retry.mutate(undefined, {
                onSuccess: (res) =>
                  notifications.show({
                    message: `Retrying ${res.queued ?? 0} screenshot${res.queued === 1 ? '' : 's'}`,
                  }),
                onError: (err) => {
                  notifications.show({ color: 'red', message: String(err) })
                  void status.refetch()
                },
              })
            }
          >
            Retry failed screenshots
          </Button>
        </Group>
        <RangeInput
          label="Retry cooldown"
          description="Time between manual retries. A retry takes only screenshots that failed longer ago than this."
          value={settings.screenshot_retry_cooldown_hours}
          min={1}
          max={720}
          suffix=" hours"
          disabled={save.isPending}
          onSave={(screenshot_retry_cooldown_hours) => saveSetting({ screenshot_retry_cooldown_hours })}
        />
        <RangeInput
          label="Retry automatically after"
          description="Failed screenshots older than this are retried by the check that runs every 6 hours."
          value={settings.screenshot_auto_retry_days}
          min={1}
          max={90}
          suffix=" days"
          disabled={save.isPending}
          onSave={(screenshot_auto_retry_days) => saveSetting({ screenshot_auto_retry_days })}
        />
      </Stack>
    </Paper>
  )
}
