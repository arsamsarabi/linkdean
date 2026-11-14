import { useMantineTheme } from '@mantine/core'
import { useMediaQuery } from '@mantine/hooks'
import { useShellUi } from '@/stores'

/** True on sm+ when the sidebar is collapsed to the icon rail. Phones keep the full drawer. */
export function useIconRail(): boolean {
  const navCollapsed = useShellUi((s) => s.navCollapsed)
  const theme = useMantineTheme()
  const smUp = useMediaQuery(`(min-width: ${theme.breakpoints.sm})`)
  return Boolean(smUp && navCollapsed)
}
