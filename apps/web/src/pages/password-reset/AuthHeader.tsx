import type { ReactElement } from 'react'
import { Group, Stack, Text, Title } from '@mantine/core'
import { LogoMark } from '@/components'

/** The logo, title and subtitle above the signed-out password forms. */
export function AuthHeader({ title, subtitle }: { title: string; subtitle: string }): ReactElement {
  return (
    <Stack gap={6}>
      <Group gap="sm" wrap="nowrap">
        <LogoMark size={40} />
        <Title order={1} c="violet">
          {title}
        </Title>
      </Group>
      <Text c="dimmed">{subtitle}</Text>
    </Stack>
  )
}
