import { useId, useState, type ReactElement } from 'react'
import { Badge, Collapse, Group, Popover, UnstyledButton } from '@mantine/core'
import { IconChevronDown, IconTag } from '@tabler/icons-react'
import styles from './BookmarkCard.module.css'

type Props = {
  tags: string[]
  /** `card` opens the tags under the toggle; `row` opens them in a popover so the row keeps its height. */
  variant?: 'card' | 'row'
}

function tagCount(n: number): string {
  return n === 1 ? '1 tag' : `${n} tags`
}

/** A bookmark's tags behind a "N tags" toggle, closed at first, so long tag lists don't make the item taller. */
export function BookmarkCardTags({ tags, variant = 'card' }: Props): ReactElement | null {
  const [open, setOpen] = useState(false)
  const listId = useId()
  if (tags.length === 0) return null

  const list = (
    <Group gap={4} id={listId}>
      {tags.map((t) => (
        <Badge key={t} variant="outline" size="sm">
          {t}
        </Badge>
      ))}
    </Group>
  )

  const toggle = (
    <UnstyledButton
      className={styles.tagToggle}
      aria-expanded={open}
      aria-controls={variant === 'card' ? listId : undefined}
      onClick={(e) => {
        e.stopPropagation()
        setOpen((o) => !o)
      }}
    >
      <IconTag size={12} stroke={1.75} aria-hidden />
      {tagCount(tags.length)}
      <IconChevronDown
        size={12}
        stroke={1.75}
        aria-hidden
        className={styles.tagChevron}
        data-open={open || undefined}
      />
    </UnstyledButton>
  )

  if (variant === 'row') {
    return (
      <Popover opened={open} onChange={setOpen} position="bottom-end" shadow="md" withinPortal>
        <Popover.Target>{toggle}</Popover.Target>
        <Popover.Dropdown maw={320}>{list}</Popover.Dropdown>
      </Popover>
    )
  }

  return (
    <div>
      {toggle}
      <Collapse expanded={open}>
        <div className={styles.tagList}>{list}</div>
      </Collapse>
    </div>
  )
}
