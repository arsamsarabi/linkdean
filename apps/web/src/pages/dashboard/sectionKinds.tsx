import { type CSSProperties, type ReactElement } from 'react'
import { IconClockHour4, IconFlame, IconHistory, IconPinned } from '@tabler/icons-react'
import type { DashboardSectionKind } from '@/schemas'

export type SectionKindLook = {
  label: string
  /** A theme color: each kind has its own, so sections read apart at a glance. */
  color: 'violet' | 'ice' | 'yellow' | 'green'
  icon: (size: number) => ReactElement
}

export const SECTION_KIND_LOOK: Record<DashboardSectionKind, SectionKindLook> = {
  pinned: { label: 'Pinned', color: 'violet', icon: (size) => <IconPinned size={size} stroke={1.6} /> },
  latest: { label: 'Latest', color: 'ice', icon: (size) => <IconClockHour4 size={size} stroke={1.6} /> },
  most_opened: { label: 'Most opened', color: 'yellow', icon: (size) => <IconFlame size={size} stroke={1.6} /> },
  recently_visited: {
    label: 'Recently visited',
    color: 'green',
    icon: (size) => <IconHistory size={size} stroke={1.6} />,
  },
}

/** The CSS variables that give a section panel its kind's color in both themes. */
export function kindAccent(kind: DashboardSectionKind): CSSProperties {
  const color = SECTION_KIND_LOOK[kind].color
  return {
    '--dash-accent-light': `var(--mantine-color-${color}-6)`,
    '--dash-accent-dark': `var(--mantine-color-${color}-4)`,
  } as CSSProperties
}
