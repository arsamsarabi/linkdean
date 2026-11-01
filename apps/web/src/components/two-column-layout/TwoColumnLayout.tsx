import type { ReactElement, ReactNode } from 'react'
import { SimpleGrid, Stack } from '@mantine/core'

type Props = {
  left: ReactNode
  right: ReactNode
}

/** Two independent card columns from `md` up, one column below. */
export function TwoColumnLayout({ left, right }: Props): ReactElement {
  return (
    <SimpleGrid cols={{ base: 1, md: 2 }} spacing="lg" verticalSpacing="lg">
      <Stack gap="lg" miw={0}>
        {left}
      </Stack>
      <Stack gap="lg" miw={0}>
        {right}
      </Stack>
    </SimpleGrid>
  )
}
