import { type ReactElement } from 'react'
import { Skeleton, Stack } from '@mantine/core'

export function SettingsSkeleton(): ReactElement {
  return (
    <Stack gap="md" maw={560}>
      <Skeleton height={48} />
      <Skeleton height={64} />
      <Skeleton height={64} />
      <Skeleton height={48} />
    </Stack>
  )
}
