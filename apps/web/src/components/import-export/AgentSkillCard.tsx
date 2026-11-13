import { useMemo, useState, type ReactElement } from 'react'
import {
  Anchor,
  Button,
  Code,
  Group,
  Paper,
  SegmentedControl,
  Select,
  Stack,
  Text,
  ThemeIcon,
  Title,
} from '@mantine/core'
import { IconDownload, IconRobot } from '@tabler/icons-react'
import { useCategories, useDownloadAgentSkill, useTagNames } from '@/hooks'
import { categoryPath } from '@/lib'

type From = 'folder' | 'tag'

/**
 * Config options → Import / Export: turn a folder (with its subfolders) or a tag into an Agent Skill
 * for coding and research agents, and show how to fetch it with an API key.
 */
export function AgentSkillCard(): ReactElement {
  const [from, setFrom] = useState<From>('folder')
  const [folder, setFolder] = useState<string | null>(null)
  const [tag, setTag] = useState<string | null>(null)
  const categories = useCategories(true, false)
  const tags = useTagNames()
  const { download, pending } = useDownloadAgentSkill()

  const folderData = useMemo(() => {
    const cats = (categories.data ?? []).filter((c) => !c.is_vault && !c.deleted_at)
    const byId = new Map(cats.map((c) => [c.id, c]))
    return cats
      .map((c) => ({ value: c.id, label: categoryPath(c, byId) }))
      .sort((a, b) => a.label.localeCompare(b.label))
  }, [categories.data])

  const source = from === 'folder' ? (folder ? { folder } : null) : tag ? { tag } : null
  const query = from === 'folder' ? `folder=${folder ?? '<folder id>'}` : `tag=${encodeURIComponent(tag ?? '<tag>')}`

  return (
    <Paper p="xl" withBorder>
      <Stack gap="lg">
        <Group gap="sm">
          <ThemeIcon size={28} radius="md" variant="light" color="violet">
            <IconRobot size={16} stroke={1.5} />
          </ThemeIcon>
          <Title order={4}>Agent Skill</Title>
        </Group>
        <Text size="sm" c="dimmed">
          Give coding and research agents (Claude Code, Codex, Cursor and others) your curated links. A folder or a tag
          becomes an{' '}
          <Anchor href="https://agentskills.io" target="_blank" rel="noreferrer">
            Agent Skill
          </Anchor>
          : unzip it into the agent&apos;s skills folder, such as <Code>~/.claude/skills</Code>. Vault bookmarks are
          never included.
        </Text>
        <SegmentedControl
          value={from}
          onChange={(v) => setFrom(v as From)}
          data={[
            { value: 'folder', label: 'Folder' },
            { value: 'tag', label: 'Tag' },
          ]}
          aria-label="Make the skill from"
        />
        {from === 'folder' ? (
          <Select
            label="Folder"
            description="Its subfolders are included."
            placeholder="Pick a folder"
            searchable
            data={folderData}
            value={folder}
            onChange={setFolder}
          />
        ) : (
          <Select
            label="Tag"
            description="Your own bookmarks with this tag."
            placeholder="Pick a tag"
            searchable
            data={tags.data ?? []}
            value={tag}
            onChange={setTag}
          />
        )}
        <Group>
          <Button
            variant="light"
            leftSection={<IconDownload size={16} stroke={1.5} />}
            disabled={!source}
            loading={pending}
            onClick={() => source && void download(source)}
          >
            Download skill
          </Button>
        </Group>
        <Text size="xs" c="dimmed">
          To keep it current, fetch it with an API key (Config options → API keys), for example from a script:
        </Text>
        <Code
          block
          style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-all' }}
        >{`curl -H "Authorization: Bearer ldk_…" \\\n  "${window.location.origin}/api/agent-skill?${query}" -o skill.zip`}</Code>
      </Stack>
    </Paper>
  )
}
