import { type ReactElement } from 'react'
import { ActionIcon, Affix, Group, Tooltip, Transition } from '@mantine/core'
import { IconArrowUp, IconBookmarkPlus } from '@tabler/icons-react'
import { useLibraryPageModel } from './useLibraryPageModel'

export function LibraryFab(): ReactElement {
  const lib = useLibraryPageModel()

  return (
    <Affix position={{ bottom: 24, right: 24 }}>
      <Group gap="sm">
        <Transition transition="slide-up" mounted={lib.scroll.y > 320}>
          {(transitionStyles) => (
            <Tooltip label="Back to top">
              <ActionIcon
                style={transitionStyles}
                size="lg"
                radius="xl"
                variant="filled"
                color="violet"
                aria-label="Back to top"
                onClick={() => window.scrollTo({ top: 0, behavior: 'smooth' })}
              >
                <IconArrowUp size={18} stroke={1.5} />
              </ActionIcon>
            </Tooltip>
          )}
        </Transition>
        {lib.canCreateInContext ? (
          <Tooltip label="Add bookmark" position="left">
            <ActionIcon
              size="xl"
              radius="xl"
              variant="filled"
              color="violet"
              aria-label="Add bookmark"
              onClick={lib.openCreate}
              hiddenFrom="sm"
            >
              <IconBookmarkPlus size={22} stroke={1.5} />
            </ActionIcon>
          </Tooltip>
        ) : null}
      </Group>
    </Affix>
  )
}
