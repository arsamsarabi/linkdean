import type { ReactElement } from 'react'
import { Card, Group, SimpleGrid, Skeleton, Stack } from '@mantine/core'
import styles from './BookmarkCard.module.css'

export function BookmarkCardSkeleton(): ReactElement {
  return (
    <Card withBorder padding="sm" radius="md" h="100%" className={styles.card}>
      <Card.Section>
        <Skeleton height={140} radius={0} />
      </Card.Section>
      <Stack gap={8} mt="sm" className={styles.body}>
        <Skeleton height={16} width="85%" />
        <Skeleton height={12} width="45%" />
        <Skeleton height={12} width="70%" />
        <Group gap={6} mt="auto">
          <Skeleton height={20} width={52} radius="xl" />
          <Skeleton height={20} width={44} radius="xl" />
        </Group>
      </Stack>
    </Card>
  )
}

export function BookmarkGridSkeleton({ count = 8 }: { count?: number }): ReactElement {
  return (
    <SimpleGrid cols={{ base: 1, sm: 2, lg: 3, xl: 4 }} spacing="lg">
      {Array.from({ length: count }, (_, i) => (
        <BookmarkCardSkeleton key={i} />
      ))}
    </SimpleGrid>
  )
}
