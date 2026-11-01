import { useState, type ReactElement } from 'react'
import { Anchor, Card, Group, Image, Loader, Pagination, Pill, SimpleGrid, Stack, Text } from '@mantine/core'
import { QueryErrorAlert } from '@/components/query-error-alert'
import { PROFILE_PAGE_SIZE, useProfileBookmarks } from '@/hooks'

type Props = {
  userId: string
  emptyLabel: string
}

export function ProfileBookmarksList({ userId, emptyLabel }: Props): ReactElement {
  const [page, setPage] = useState(1)
  const bookmarks = useProfileBookmarks(userId, page)

  if (bookmarks.isLoading) return <Loader size="sm" />
  if (bookmarks.isError) {
    return <QueryErrorAlert error={bookmarks.error} onRetry={() => void bookmarks.refetch()} />
  }

  const items = bookmarks.data?.bookmarks ?? []
  const total = bookmarks.data?.total ?? 0
  const pages = Math.ceil(total / PROFILE_PAGE_SIZE)

  if (items.length === 0) return <Text c="dimmed">{emptyLabel}</Text>

  return (
    <Stack gap="md">
      <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="md">
        {items.map((b) => (
          <Card key={b.id} withBorder padding="md" radius="md">
            <Stack gap="xs">
              <Group gap="xs" wrap="nowrap">
                {b.favicon_url ? <Image src={b.favicon_url} w={16} h={16} alt="" /> : null}
                <Anchor href={b.url} target="_blank" rel="noreferrer" lineClamp={1} fw={600}>
                  {b.title || b.url}
                </Anchor>
              </Group>
              {b.description ? (
                <Text size="sm" c="dimmed" lineClamp={2}>
                  {b.description}
                </Text>
              ) : null}
              {b.tags.length > 0 ? (
                <Group gap={4}>
                  {b.tags.map((tag) => (
                    <Pill key={tag} size="xs">
                      {tag}
                    </Pill>
                  ))}
                </Group>
              ) : null}
            </Stack>
          </Card>
        ))}
      </SimpleGrid>
      {pages > 1 ? <Pagination total={pages} value={page} onChange={setPage} aria-label="Bookmark pages" /> : null}
    </Stack>
  )
}
