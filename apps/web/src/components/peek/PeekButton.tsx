import type { MouseEvent, ReactElement } from 'react'
import { ActionIcon, Tooltip } from '@mantine/core'
import { IconEye } from '@tabler/icons-react'
import { canPeekBookmark } from '@/stores'
import { useOpenPeek, type PeekableBookmark } from './useOpenPeek'

type Props = {
  bookmark: PeekableBookmark
  className?: string
  /** Compact tiles use a smaller hit target. */
  size?: number
}

/** Explicit peek control. Stops the parent link from opening a new tab. */
export function PeekButton({ bookmark, className, size = 16 }: Props): ReactElement | null {
  const openPeek = useOpenPeek()
  if (!canPeekBookmark(bookmark)) return null

  function onClick(e: MouseEvent): void {
    e.preventDefault()
    e.stopPropagation()
    openPeek(bookmark)
  }

  return (
    <Tooltip label="Peek">
      <ActionIcon
        className={className}
        variant="subtle"
        color="gray"
        size="sm"
        aria-label={`Peek ${bookmark.title.trim() || 'bookmark'}`}
        onClick={onClick}
      >
        <IconEye size={size} stroke={1.5} />
      </ActionIcon>
    </Tooltip>
  )
}
