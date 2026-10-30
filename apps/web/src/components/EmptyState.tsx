import { Button, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import type { ReactNode } from 'react'

export function EmptyState({
  icon,
  title,
  description,
  action,
}: {
  icon: ReactNode
  title: string
  description?: string
  action?: { label: string; onClick: () => void; leftSection?: ReactNode }
}) {
  return (
    <Stack align="center" gap="md" py={64} px="md" className="ld-fade-in">
      <ThemeIcon size={56} radius="xl" variant="light" color="harbor">
        {icon}
      </ThemeIcon>
      <Stack gap={6} align="center" maw={360}>
        <Title order={3} ta="center">
          {title}
        </Title>
        {description ? (
          <Text c="dimmed" size="sm" ta="center">
            {description}
          </Text>
        ) : null}
      </Stack>
      {action ? (
        <Button leftSection={action.leftSection} onClick={action.onClick}>
          {action.label}
        </Button>
      ) : null}
    </Stack>
  )
}
