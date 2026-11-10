import { type ReactElement } from 'react'
import { Group, Text, ThemeIcon, Title } from '@mantine/core'
import type { Icon } from '@tabler/icons-react'

type Props = {
  title: string
  description: string
  icon: Icon
  color: string
}

export function LibraryHealthHeader({ title, description, icon: IconComponent, color }: Props): ReactElement {
  return (
    <Group gap="md" wrap="nowrap">
      <ThemeIcon size={40} radius="md" variant="light" color={color}>
        <IconComponent size={22} stroke={1.5} />
      </ThemeIcon>
      <div>
        <Title order={2}>{title}</Title>
        <Text size="sm" c="dimmed">
          {description}
        </Text>
      </div>
    </Group>
  )
}
