import { type ReactElement } from 'react'
import { Drawer } from '@mantine/core'
import { useDrawerClosesOnEscape, useDrawerSize } from '@/hooks/render'
import type { SocialSubjectType } from '@/schemas'
import { SocialThread } from './SocialThread'

type Props = {
  opened: boolean
  onClose: () => void
  subjectType: SocialSubjectType
  subjectId: string
  title?: string
}

export function SocialDrawer({ opened, onClose, subjectType, subjectId, title }: Props): ReactElement {
  const closeOnEscape = useDrawerClosesOnEscape()
  const drawerSize = useDrawerSize()
  return (
    <Drawer
      opened={opened}
      onClose={onClose}
      closeOnEscape={closeOnEscape}
      position="right"
      size={drawerSize}
      title={title?.trim() || 'Shared activity'}
      padding="md"
    >
      <SocialThread subjectType={subjectType} subjectId={subjectId} enabled={opened} />
    </Drawer>
  )
}
