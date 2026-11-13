import { useMediaQuery } from '@mantine/hooks'

/** The usual side-drawer width on tablets and desktops: a little wider than Mantine's `md` (440px). */
export const DRAWER_WIDTH = 520

/** Full-width drawers on phones; keep the desktop size from `sm` up. */
export function useDrawerSize(desktop: string | number = DRAWER_WIDTH): string | number {
  const narrow = useMediaQuery('(max-width: 48em)')
  return narrow ? '100%' : desktop
}
