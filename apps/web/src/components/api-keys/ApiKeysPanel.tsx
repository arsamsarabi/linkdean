import { useState, type ReactElement } from 'react'
import {
  Alert,
  Button,
  Checkbox,
  Code,
  Group,
  Paper,
  SegmentedControl,
  Skeleton,
  Stack,
  Text,
  TextInput,
  Title,
} from '@mantine/core'
import { notifications } from '@mantine/notifications'
import { IconCopy, IconKey, IconPlus, IconTrash } from '@tabler/icons-react'
import { ConfirmModal, QueryErrorAlert } from '@/components'
import { useApiKeys, useCreateApiKey, useMe, useRevokeApiKey } from '@/hooks'

const iconSm = { size: 16, stroke: 1.5 } as const

/** Keep in sync with `ALL_SCOPES` / `API_KEY_TEMPLATES` in crates/core/src/api_scopes.rs */
const ALL_SCOPES = [
  'library:read',
  'library:write',
  'archive:read',
  'archive:write',
  'import_export:read',
  'import_export:write',
  'social:read',
  'social:write',
  'account:read',
  'account:write',
] as const

const TEMPLATES = [
  { id: 'read_only_library', label: 'Read-only library', scopes: ['library:read'] },
  { id: 'save_bookmarks', label: 'Save bookmarks', scopes: ['library:read', 'library:write'] },
  { id: 'full_access', label: 'Full access', scopes: [...ALL_SCOPES] },
] as const

type TemplateId = (typeof TEMPLATES)[number]['id'] | 'custom'

function scopesEqual(a: string[], b: readonly string[]): boolean {
  if (a.length !== b.length) return false
  const set = new Set(a)
  return b.every((s) => set.has(s))
}

function matchingTemplateId(scopes: string[]): TemplateId {
  for (const t of TEMPLATES) {
    if (scopesEqual(scopes, t.scopes)) return t.id
  }
  return 'custom'
}

export function ApiKeysPanel(): ReactElement {
  const me = useMe()
  const allowed = me.data?.api_keys_allowed !== false
  const keys = useApiKeys(allowed)
  const create = useCreateApiKey()
  const revoke = useRevokeApiKey()
  const [name, setName] = useState('')
  const [template, setTemplate] = useState<TemplateId>('full_access')
  const [scopes, setScopes] = useState<string[]>([...ALL_SCOPES])
  const [freshToken, setFreshToken] = useState<string | null>(null)
  const [revokeId, setRevokeId] = useState<string | null>(null)

  function applyTemplate(id: TemplateId): void {
    setTemplate(id)
    if (id === 'custom') return
    const t = TEMPLATES.find((x) => x.id === id)
    if (t) setScopes([...t.scopes])
  }

  function onScopesChange(next: string[]): void {
    setScopes(next)
    setTemplate(matchingTemplateId(next))
  }

  async function onCreate(): Promise<void> {
    const trimmed = name.trim()
    if (!trimmed || create.isPending || scopes.length === 0) return
    try {
      const res = await create.mutateAsync({ name: trimmed, scopes })
      setName('')
      setTemplate('full_access')
      setScopes([...ALL_SCOPES])
      setFreshToken(res.token)
      notifications.show({ message: 'API key created — copy it now' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  async function onRevoke(): Promise<void> {
    if (!revokeId) return
    try {
      await revoke.mutateAsync(revokeId)
      setRevokeId(null)
      notifications.show({ message: 'API key revoked' })
    } catch (e) {
      notifications.show({ color: 'red', message: String(e) })
    }
  }

  if (!allowed) {
    return (
      <Alert color="yellow" title="API keys disabled">
        An admin turned off API keys for this account or the whole platform.
      </Alert>
    )
  }

  return (
    <Stack gap="lg" maw={640}>
      <Alert color="violet" variant="light" icon={<IconKey {...iconSm} />}>
        Keys authenticate as you via <Code>Authorization: Bearer ldk_…</Code>. Each key only gets the permissions you
        tick. Secret shown once at create.
      </Alert>

      {freshToken ? (
        <Paper p="md" withBorder>
          <Stack gap="sm">
            <Text size="sm" fw={500}>
              New key (copy now — not shown again)
            </Text>
            <Group gap="sm" wrap="nowrap" align="flex-start">
              <Code block style={{ flex: 1, wordBreak: 'break-all' }}>
                {freshToken}
              </Code>
              <Button
                variant="light"
                leftSection={<IconCopy {...iconSm} />}
                onClick={() => {
                  void navigator.clipboard.writeText(freshToken)
                  notifications.show({ message: 'Copied' })
                }}
              >
                Copy
              </Button>
            </Group>
            <Button variant="subtle" size="xs" onClick={() => setFreshToken(null)}>
              Dismiss
            </Button>
          </Stack>
        </Paper>
      ) : null}

      <Paper p="xl" withBorder>
        <Stack gap="md">
          <Title order={4}>Create key</Title>
          <TextInput
            label="Name"
            placeholder="e.g. CLI, script, CI"
            value={name}
            onChange={(e) => setName(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') void onCreate()
            }}
          />
          <Stack gap="xs">
            <Text size="sm" fw={500}>
              Template
            </Text>
            <Text size="xs" c="dimmed">
              Preset fills permissions; you can still tweak the boxes.
            </Text>
            <SegmentedControl
              fullWidth
              value={template}
              onChange={(v) => applyTemplate(v as TemplateId)}
              data={[...TEMPLATES.map((t) => ({ value: t.id, label: t.label })), { value: 'custom', label: 'Custom' }]}
              aria-label="Permission template"
            />
          </Stack>
          <Checkbox.Group
            label="Permissions"
            description="Read vs write for each area. At least one required."
            value={scopes}
            onChange={onScopesChange}
          >
            <Stack gap="xs" mt="xs">
              {ALL_SCOPES.map((scope) => (
                <Checkbox key={scope} value={scope} label={scope} />
              ))}
            </Stack>
          </Checkbox.Group>
          <Group>
            <Button
              leftSection={<IconPlus {...iconSm} />}
              loading={create.isPending}
              disabled={!name.trim() || scopes.length === 0}
              onClick={() => void onCreate()}
            >
              Create
            </Button>
          </Group>
        </Stack>
      </Paper>

      <Paper p="xl" withBorder>
        <Stack gap="md">
          <Title order={4}>Active keys</Title>
          {keys.isError ? <QueryErrorAlert error={keys.error} onRetry={() => void keys.refetch()} /> : null}
          {keys.isLoading ? <Skeleton height={48} /> : null}
          {!keys.isLoading && (keys.data?.keys.length ?? 0) === 0 ? (
            <Text size="sm" c="dimmed">
              No keys yet.
            </Text>
          ) : null}
          {(keys.data?.keys ?? []).map((k) => (
            <Group key={k.id} justify="space-between" wrap="nowrap" align="flex-start">
              <Stack gap={2} style={{ minWidth: 0 }}>
                <Text size="sm" fw={500} truncate>
                  {k.name}
                </Text>
                <Text size="xs" c="dimmed">
                  {k.prefix}… · created {new Date(k.created_at).toLocaleString()}
                  {k.last_used_at ? ` · last used ${new Date(k.last_used_at).toLocaleString()}` : ' · never used'}
                </Text>
                <Text size="xs" c="dimmed" style={{ wordBreak: 'break-word' }}>
                  {(k.scopes ?? []).join(', ') || 'no scopes'}
                </Text>
              </Stack>
              <Button
                color="red"
                variant="light"
                size="xs"
                leftSection={<IconTrash {...iconSm} />}
                loading={revoke.isPending && revokeId === k.id}
                onClick={() => setRevokeId(k.id)}
              >
                Revoke
              </Button>
            </Group>
          ))}
        </Stack>
      </Paper>

      <ConfirmModal
        opened={!!revokeId}
        onClose={() => setRevokeId(null)}
        title="Revoke API key?"
        body="Scripts using this key stop working immediately."
        confirmLabel="Revoke"
        confirmColor="red"
        loading={revoke.isPending}
        onConfirm={() => void onRevoke()}
      />
    </Stack>
  )
}
