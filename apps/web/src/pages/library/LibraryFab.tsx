import { type ReactElement } from 'react'
import { ActionIcon, Affix, Group, Tooltip, Transition } from '@mantine/core'
import { IconArrowUp } from '@tabler/icons-react'
import { useScrollRoot } from '@/hooks/render'
import { useShellUi } from '@/stores'
import { useLibraryPageModel } from './useLibraryPageModel'

export function LibraryFab(): ReactElement {
  const lib = useLibraryPageModel()
  const scrollRoot = useScrollRoot()
  // The phone menu slides over the page; keep the floating buttons out of it.
  const navOpen = useShellUi((s) => s.navOpen)
  if (navOpen) return <></>

  // On phones the tab bar sits at the bottom; stay above it.
  return (
    <Affix position={{ bottom: 'calc(24px + var(--app-shell-footer-offset, 0px))', right: 24 }}>
      <Group gap="sm">
        <Transition transition="slide-up" mounted={lib.scrollY > 320}>
          {(transitionStyles) => (
            <Tooltip label="Back to top">
              <ActionIcon
                style={transitionStyles}
                size="lg"
                radius="xl"
                variant="filled"
                color="violet"
                aria-label="Back to top"
                onClick={() => (scrollRoot ?? window).scrollTo({ top: 0, behavior: 'smooth' })}
              >
                <IconArrowUp size={18} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          )}
        </Transition>
      </Group>
    </Affix>
  )
}
