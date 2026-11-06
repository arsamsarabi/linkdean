import { useMediaQuery } from '@mantine/hooks'

/** Full-width drawers on phones; keep the desktop size from `sm` up. */
export function useDrawerSize(desktop: string | number = 'md'): string | number {
  const narrow = useMediaQuery('(max-width: 48em)')
  return narrow ? '100%' : desktop
}
