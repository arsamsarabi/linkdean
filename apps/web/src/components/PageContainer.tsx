import { Container, type ContainerProps } from '@mantine/core'

/** Shared max-width for AppShell header + main content. */
export const PAGE_MAX_WIDTH = 1140

export function PageContainer({
  children,
  ...props
}: ContainerProps) {
  return (
    <Container size={PAGE_MAX_WIDTH} px="md" w="100%" {...props}>
      {children}
    </Container>
  )
}
