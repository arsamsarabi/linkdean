import { type ReactElement } from 'react'
import { Anchor, Badge, Group, Indicator, Text, Tooltip } from '@mantine/core'
import {
  IconAlertOctagon,
  IconAlertTriangle,
  IconArrowUp,
  IconCircleCheck,
  IconSparkles,
  type Icon,
} from '@tabler/icons-react'
import { useAppVersion, useUpdateStatus } from '@/hooks'
import type { UpdateStatus } from '@/schemas'

type Level = NonNullable<UpdateStatus['update']>['level']

/** One look per kind of release: color and icon get stronger with the risk of updating. */
const LEVELS: Record<Level, { color: string; Icon: Icon; label: string; hint: string }> = {
  patch: { color: 'blue', Icon: IconArrowUp, label: 'Patch', hint: 'Bug fixes. Safe to update.' },
  minor: { color: 'teal', Icon: IconSparkles, label: 'New features', hint: 'New features. Safe to update.' },
  major: {
    color: 'orange',
    Icon: IconAlertTriangle,
    label: 'Major',
    hint: 'A major release. Read the release notes before you update.',
  },
  breaking: {
    color: 'red',
    Icon: IconAlertOctagon,
    label: 'Breaking',
    hint: 'Has breaking changes. Read the release notes first: updating may need manual steps.',
  },
}

type Props = {
  isAdmin: boolean
  /** Icon rail: the version only, with a colored dot when an update is out. */
  compact?: boolean
}

/**
 * The running version at the foot of the sidebar. Admins also see whether a newer release is
 * out, and how risky it is to update; the badge opens its release notes.
 */
export function SidebarVersion({ isAdmin, compact = false }: Props): ReactElement | null {
  const version = useAppVersion()
  const status = useUpdateStatus(isAdmin)
  if (!version) return null

  const update = status.data?.update
  const current = `v${version}`

  if (!update) {
    const text = (
      <Text size="xs" c="dimmed" ta={compact ? 'center' : undefined}>
        {current}
      </Text>
    )
    if (!isAdmin || !status.isSuccess) return text
    return (
      <Tooltip label="Linkdean is up to date" withArrow>
        <Group gap={4} justify={compact ? 'center' : undefined} wrap="nowrap">
          {text}
          {compact ? null : <IconCircleCheck size={14} stroke={1.5} color="var(--mantine-color-dimmed)" />}
        </Group>
      </Tooltip>
    )
  }

  const look = LEVELS[update.level]
  const tip = `Linkdean v${update.latest} is out. ${look.hint}`
  const linkProps = {
    href: update.url,
    target: '_blank',
    rel: 'noopener noreferrer',
    'aria-label': `${current}. ${tip} Open the release notes.`,
  } as const

  if (compact) {
    return (
      <Tooltip label={tip} withArrow position="right" multiline w={220}>
        <Anchor {...linkProps} underline="never" c="dimmed" size="xs" ta="center" display="block">
          <Indicator color={look.color} size={8} offset={-2} processing={update.level === 'breaking'}>
            {current}
          </Indicator>
        </Anchor>
      </Tooltip>
    )
  }

  return (
    <Group justify="space-between" gap="xs" wrap="nowrap">
      <Text size="xs" c="dimmed">
        {current}
      </Text>
      <Tooltip label={tip} withArrow multiline w={240}>
        <Badge
          component="a"
          {...linkProps}
          color={look.color}
          variant={update.level === 'patch' || update.level === 'minor' ? 'light' : 'filled'}
          size="sm"
          leftSection={<look.Icon size={12} stroke={2} />}
          style={{ cursor: 'pointer' }}
        >
          v{update.latest} · {look.label}
        </Badge>
      </Tooltip>
    </Group>
  )
}
