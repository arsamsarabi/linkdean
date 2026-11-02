import { useMemo, type ReactElement } from 'react'
import { Anchor, Center, Group, Loader, SimpleGrid, Stack, Text, Title } from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from '@tanstack/react-router'
import { api } from '@/api'
import { BookmarkCard, LogoMark, PageContainer, QueryErrorAlert } from '@/components'
import { CategoryIcon } from '@/lib'
import motion from '@/styles/motion.module.css'

export function PublicListPage(): ReactElement {
  const { slug } = useParams({ from: '/l/$slug' })
  const list = useQuery({
    queryKey: ['public-list', slug],
    queryFn: () => api.publicList(slug),
    enabled: !!slug,
  })

  const cards = useMemo(
    () =>
      (list.data?.bookmarks || []).map((b) => (
        <BookmarkCard
          key={b.id}
          bookmark={{
            id: b.id,
            url: b.url,
            title: b.title,
            description: b.description,
            favicon_url: b.favicon_url,
            preview_image_url: b.preview_image_url,
            has_preview: b.has_preview,
            preview_url: b.preview_url,
            is_vault: false,
            tags: b.tags,
            can_write: false,
          }}
          onEdit={() => undefined}
          onDelete={() => undefined}
        />
      )),
    [list.data?.bookmarks]
  )

  if (list.isLoading) {
    return (
      <Center mih="70vh">
        <Loader color="violet" type="dots" />
      </Center>
    )
  }

  if (list.isError) {
    return (
      <PageContainer py="xl">
        <Stack gap="md" maw={480} mx="auto">
          <LogoMark />
          <QueryErrorAlert error={list.error} onRetry={() => void list.refetch()} />
          <Text size="sm" c="dimmed">
            This public list may have been revoked or the link is wrong.
          </Text>
          <Anchor component={Link} to="/login" size="sm">
            Sign in to Linkdean
          </Anchor>
        </Stack>
      </PageContainer>
    )
  }

  const data = list.data!
  const total = data.total ?? data.bookmarks.length

  return (
    <PageContainer py="xl" fadeIn>
      <Stack gap="xl" className={motion.fadeIn}>
        <Group justify="space-between" align="flex-start" wrap="wrap">
          <Stack gap={4}>
            <Group gap="xs">
              <CategoryIcon icon={data.icon} size={22} />
              <Title order={2}>{data.name}</Title>
            </Group>
            <Text size="sm" c="dimmed">
              Shared by {data.owner_display_name} · {total} bookmark
              {total === 1 ? '' : 's'}
              {total > data.bookmarks.length ? ` · showing the newest ${data.bookmarks.length}` : ''}
            </Text>
          </Stack>
          <Anchor component={Link} to="/login" size="sm">
            Sign in
          </Anchor>
        </Group>

        {data.bookmarks.length === 0 ? (
          <Text c="dimmed">This list is empty.</Text>
        ) : (
          <SimpleGrid cols={{ base: 1, sm: 2, lg: 3 }} spacing="md">
            {cards}
          </SimpleGrid>
        )}
      </Stack>
    </PageContainer>
  )
}
