import type { ReactElement } from 'react'
import { Container, type ContainerProps } from '@mantine/core'
import styles from './PageContainer.module.css'

/** Shared max-width for AppShell header + main content. */
export const PAGE_MAX_WIDTH = 1440

type Props = ContainerProps & {
  fadeIn?: boolean
}

export function PageContainer({ children, fadeIn, className, ...props }: Props): ReactElement {
  const cls = [styles.root, fadeIn ? styles.fadeIn : null, className].filter(Boolean).join(' ')
  return (
    <Container size={PAGE_MAX_WIDTH} px="md" className={cls} {...props}>
      {children}
    </Container>
  )
}
