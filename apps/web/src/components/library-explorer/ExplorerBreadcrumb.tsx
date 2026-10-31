import type { ReactElement } from 'react'
import { Anchor, Breadcrumbs, Text } from '@mantine/core'
import type { BreadcrumbSegment } from '@/services'

type Props = {
  segments: BreadcrumbSegment[]
  onNavigate: (id: string) => void
}

export function ExplorerBreadcrumb({ segments, onNavigate }: Props): ReactElement {
  return (
    <Breadcrumbs aria-label="Folder path">
      {segments.map((seg, i) => {
        const last = i === segments.length - 1
        if (last) {
          return (
            <Text key={seg.id} size="sm" fw={600}>
              {seg.name}
            </Text>
          )
        }
        return (
          <Anchor key={seg.id} component="button" type="button" size="sm" onClick={() => onNavigate(seg.id)}>
            {seg.name}
          </Anchor>
        )
      })}
    </Breadcrumbs>
  )
}
