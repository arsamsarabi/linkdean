import type { ReactElement, ReactNode } from 'react'
import { Button, Stack, Text, ThemeIcon, Title } from '@mantine/core'
import { MASCOT_EMPTY_DISPLAY_PX, MascotFigure } from '@/components/mascot'
import styles from './EmptyState.module.css'

type Illustration = {
  src: string
  displayWidth?: number
}

type Props = {
  icon?: ReactNode
  illustration?: Illustration
  title: string
  description?: string
  action?: { label: string; onClick: () => void; leftSection?: ReactNode }
}

export function EmptyState({ icon, illustration, title, description, action }: Props): ReactElement {
  return (
    <Stack align="center" gap="md" py={64} px="md" className={styles.root}>
      {illustration ? (
        <MascotFigure src={illustration.src} width={illustration.displayWidth ?? MASCOT_EMPTY_DISPLAY_PX} />
      ) : icon ? (
        <ThemeIcon size={56} radius="xl" variant="light" color="violet">
          {icon}
        </ThemeIcon>
      ) : null}
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
