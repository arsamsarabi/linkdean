import { useMemo, type ReactElement, type ReactNode } from 'react'
import { Anchor, Badge, Card, Group, Paper, SimpleGrid, Skeleton, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import { IconFolder, IconLayoutDashboard, IconMailOpened, IconShare, IconUsers } from '@tabler/icons-react'
import { Link } from '@tanstack/react-router'
import { QueryErrorAlert, ShareInvitesList } from '@/components'
import { useCategories, useDashboards, useShareInvites } from '@/hooks'
import { buildLibrarySearch } from '@/lib'
import { splitShared, type SharedItem as Item } from './splitShared'

function Section({ title, description, children }: { title: string; description: string; children: ReactNode }) {
  return (
    <Stack gap="sm">
      <Stack gap={2}>
        <Title order={3}>{title}</Title>
        <Text size="sm" c="dimmed">
          {description}
        </Text>
      </Stack>
      {children}
    </Stack>
  )
}

function ItemCard({ item }: { item: Item }): ReactElement {
  const Icon = item.kind === 'folder' ? IconFolder : IconLayoutDashboard
  return (
    <Card withBorder padding="sm" radius="md">
      <Group wrap="nowrap" gap="sm" align="flex-start">
        <ThemeIcon variant="light" color="violet" size="lg" radius="md">
          <Icon size={18} stroke={1.5} />
        </ThemeIcon>
        <Stack gap={2} style={{ flex: 1, minWidth: 0 }}>
          <Group gap="xs" wrap="nowrap">
            <Text fw={600} truncate>
              <Anchor
                inherit
                renderRoot={(props) =>
                  item.kind === 'folder' ? (
                    <Link {...props} to="/library" search={buildLibrarySearch({ folder: item.id })} />
                  ) : (
                    <Link {...props} to="/" search={{ board: item.id }} />
                  )
                }
              >
                {item.title}
              </Anchor>
            </Text>
            {item.access ? (
              <Badge size="xs" variant="light" color={item.access === 'write' ? 'green' : 'gray'}>
                {item.access === 'write' ? 'Can edit' : 'Read only'}
              </Badge>
            ) : null}
          </Group>
          <Text size="xs" c="dimmed" lineClamp={2}>
            {item.kind === 'folder' ? 'Folder' : 'Dashboard'} · {item.people}
          </Text>
        </Stack>
      </Group>
    </Card>
  )
}

function Empty({ icon, children }: { icon: ReactNode; children: ReactNode }): ReactElement {
  return (
    <Paper withBorder p="lg" radius="md">
      <Group gap="sm" wrap="nowrap">
        <ThemeIcon variant="light" color="gray" size="lg" radius="xl">
          {icon}
        </ThemeIcon>
        <Text size="sm" c="dimmed">
          {children}
        </Text>
      </Group>
    </Paper>
  )
}

function ItemGrid({ items }: { items: Item[] }): ReactElement {
  return (
    <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="sm">
      {items.map((item) => (
        <ItemCard key={item.key} item={item} />
      ))}
    </SimpleGrid>
  )
}

/** Everything shared in one place: invites waiting, what others share with you, what you share. */
export function SharedPage(): ReactElement {
  const categories = useCategories(true)
  const dashboards = useDashboards()
  const invites = useShareInvites()
  const pending = (invites.data?.invites ?? []).filter((i) => !i.rejected_at)
  const { withMe, byMe } = useMemo(
    () => splitShared(categories.data ?? [], dashboards.data ?? []),
    [categories.data, dashboards.data]
  )
  const loading = categories.isLoading || dashboards.isLoading
  const error = categories.error || dashboards.error

  return (
    <Stack gap="xl" maw={1100}>
      <Stack gap={4}>
        <Title order={1}>Shared</Title>
        <Text c="dimmed">Invites, what others share with you, and what you share.</Text>
      </Stack>

      {error ? (
        <QueryErrorAlert
          error={error}
          onRetry={() => {
            void categories.refetch()
            void dashboards.refetch()
          }}
          title="Could not load shared items"
        />
      ) : null}

      <Section title="Waiting for you" description="Accept an invite to add the folder or dashboard to yours.">
        {pending.length > 0 ? (
          <ShareInvitesList />
        ) : (
          <Empty icon={<IconMailOpened size={18} stroke={1.5} />}>No invites waiting.</Empty>
        )}
      </Section>

      <Section title="Shared with me" description="Folders and dashboards other people share with you.">
        {loading ? (
          <Skeleton height={72} radius="md" />
        ) : withMe.length > 0 ? (
          <ItemGrid items={withMe} />
        ) : (
          <Empty icon={<IconUsers size={18} stroke={1.5} />}>Nothing is shared with you yet.</Empty>
        )}
      </Section>

      <Section
        title="Shared by me"
        description="Change who has access from the folder's menu in the library, or Share on the dashboard."
      >
        {loading ? (
          <Skeleton height={72} radius="md" />
        ) : byMe.length > 0 ? (
          <ItemGrid items={byMe} />
        ) : (
          <Empty icon={<IconShare size={18} stroke={1.5} />}>
            You have not shared anything. Share a folder from its menu in the library.
          </Empty>
        )}
      </Section>
    </Stack>
  )
}
