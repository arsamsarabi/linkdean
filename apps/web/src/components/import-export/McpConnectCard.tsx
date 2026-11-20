import type { ReactElement } from 'react'
import { Anchor, Code, Group, List, Paper, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import { IconPlugConnected } from '@tabler/icons-react'

/**
 * Config options → Import / Export: point an MCP client at this instance with an API key
 * for live library tools. Vault items are never included.
 */
export function McpConnectCard(): ReactElement {
  const mcpUrl = `${window.location.origin}/api/mcp`

  return (
    <Paper p="xl" withBorder>
      <Stack gap="lg">
        <Group gap="sm">
          <ThemeIcon size={28} radius="md" variant="light" color="violet">
            <IconPlugConnected size={16} stroke={1.5} />
          </ThemeIcon>
          <Title order={4}>MCP</Title>
        </Group>
        <Text size="sm" c="dimmed">
          Connect an agent with an API key for live search, read, and (when the key allows) create, update or trash
          bookmarks. Vault items are never included.
        </Text>
        <List size="sm" spacing={4} c="dimmed">
          <List.Item>Search and read bookmarks, folders, and tags</List.Item>
          <List.Item>Create, update, or trash bookmarks when the key has library write</List.Item>
          <List.Item>Read archived page text when a copy exists</List.Item>
          <List.Item>Vault items are never included</List.Item>
        </List>
        <Stack gap="xs">
          <Text size="sm" fw={500}>
            Endpoint
          </Text>
          <Code block style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
            {mcpUrl}
          </Code>
        </Stack>
        <Text size="sm" c="dimmed">
          <Anchor href="/settings?tab=api-keys">Create an API key</Anchor>, then send it on every request:
        </Text>
        <Code block style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}>
          {`Authorization: Bearer ldk_…`}
        </Code>
      </Stack>
    </Paper>
  )
}
