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
import { useCategories, useDownloadAgentSkill, useSavedSearches, useTagNames } from '@/hooks'
import { categoryPath } from '@/lib'

type From = 'folder' | 'tag' | 'search'

/**
 * Config options → Import / Export: turn a folder (with its subfolders), a tag, or a saved search
 * into an Agent Skill for coding and research agents, and show how to fetch it with an API key.
 */
export function AgentSkillCard(): ReactElement {
  const [from, setFrom] = useState<From>('folder')
  const [folder, setFolder] = useState<string | null>(null)
  const [tag, setTag] = useState<string | null>(null)
  const [search, setSearch] = useState<string | null>(null)
  const categories = useCategories(true, false)
  const tags = useTagNames()
  const saved = useSavedSearches()
  const { download, pending } = useDownloadAgentSkill()

  const folderData = useMemo(() => {
    const cats = (categories.data ?? []).filter((c) => !c.is_vault && !c.deleted_at)
    const byId = new Map(cats.map((c) => [c.id, c]))
    return cats
      .map((c) => ({ value: c.id, label: categoryPath(c, byId) }))
      .sort((a, b) => a.label.localeCompare(b.label))
  }, [categories.data])

  const searchData = useMemo(() => (saved.data ?? []).map((s) => ({ value: s.id, label: s.name })), [saved.data])

  const source =
    from === 'folder'
      ? folder
        ? { folder }
        : null
      : from === 'tag'
        ? tag
          ? { tag }
          : null
        : search
          ? { search }
          : null
  const query =
    from === 'folder'
      ? `folder=${folder ?? '<folder id>'}`
      : from === 'tag'
        ? `tag=${encodeURIComponent(tag ?? '<tag>')}`
        : `search=${search ?? '<saved search id>'}`

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
          Or download a static Agent Skill (.zip) for offline use. Pick a folder, a tag, or a saved search; unzip into
          the agent&apos;s skills folder, such as <Code>~/.claude/skills</Code>. See{' '}
          <Anchor href="https://agentskills.io" target="_blank" rel="noreferrer">
            agentskills.io
          </Anchor>
          . Vault bookmarks are never included.
        </Text>
        <SegmentedControl
          value={from}
          onChange={(v) => setFrom(v as From)}
          data={[
            { value: 'folder', label: 'Folder' },
            { value: 'tag', label: 'Tag' },
            { value: 'search', label: 'Saved search' },
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
        ) : from === 'tag' ? (
          <Select
            label="Tag"
            description="Your own bookmarks with this tag."
            placeholder="Pick a tag"
            searchable
            data={tags.data ?? []}
            value={tag}
            onChange={setTag}
          />
        ) : (
          <Select
            label="Saved search"
            description="Live matches of that search now. Vault bookmarks stay out."
            placeholder="Pick a saved search"
            searchable
            data={searchData}
            value={search}
            onChange={setSearch}
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
