import { type ReactElement } from 'react'
import {
  ActionIcon,
  Alert,
  Code,
  CopyButton,
  Group,
  Skeleton,
  Stack,
  Switch,
  Text,
  TextInput,
  Tooltip,
} from '@mantine/core'
import { IconCheck, IconCopy, IconRefresh } from '@tabler/icons-react'
import { QueryErrorAlert } from '@/components/query-error-alert'

type Props = {
  isVault: boolean
  loading: boolean
  error: Error | null
  onRetry: () => void
  enabled: boolean
  publicUrl: string
  publicPath: string | undefined
  busy: boolean
  rotatePending: boolean
  onToggle: (next: boolean) => void
  onRotate: () => void
}

export function CategoryPublicLinkSection({
  isVault,
  loading,
  error,
  onRetry,
  enabled,
  publicUrl,
  publicPath,
  busy,
  rotatePending,
  onToggle,
  onRotate,
}: Props): ReactElement {
  if (isVault) {
    return (
      <Alert color="gray" variant="light">
        Vault lists cannot be published. Public links never include vault bookmarks.
      </Alert>
    )
  }

  return (
    <Stack gap="sm">
      <Text size="sm" c="dimmed">
        Anyone with the link can view this list (read-only). Unlisted — not searchable. Notes stay private.
      </Text>

      {error ? <QueryErrorAlert error={error} onRetry={onRetry} /> : null}

      {loading ? (
        <Skeleton height={36} />
      ) : (
        <Switch
          label="Enable public URL"
          checked={enabled}
          disabled={busy}
          onChange={(e) => onToggle(e.currentTarget.checked)}
        />
      )}

      {enabled && publicUrl ? (
        <Stack gap="xs">
          <Group gap="xs" wrap="nowrap" align="flex-end">
            <TextInput
              label="Link"
              value={publicUrl}
              readOnly
              style={{ flex: 1 }}
              styles={{ input: { fontFamily: 'var(--mantine-font-family-monospace)' } }}
            />
            <CopyButton value={publicUrl} timeout={1500}>
              {({ copied, copy }) => (
                <Tooltip label={copied ? 'Copied' : 'Copy'}>
                  <ActionIcon
                    variant="light"
                    color={copied ? 'teal' : 'violet'}
                    aria-label="Copy public link"
                    onClick={copy}
                    mb={2}
                  >
                    {copied ? <IconCheck size={16} stroke={1.5} /> : <IconCopy size={16} stroke={1.5} />}
                  </ActionIcon>
                </Tooltip>
              )}
            </CopyButton>
            <Tooltip label="Rotate link">
              <ActionIcon
                variant="subtle"
                aria-label="Rotate public link"
                loading={rotatePending}
                onClick={onRotate}
                mb={2}
              >
                <IconRefresh size={16} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          </Group>
          <Text size="xs" c="dimmed">
            Path <Code>{publicPath}</Code>
          </Text>
        </Stack>
      ) : null}
    </Stack>
  )
}
