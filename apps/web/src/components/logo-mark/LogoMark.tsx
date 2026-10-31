import type { ReactElement } from 'react'
import styles from './LogoMark.module.css'

type Props = {
  size?: number
  alt?: string
}

export function LogoMark({ size = 28, alt = '' }: Props): ReactElement {
  return (
    <img
      src="/linkdean-symbol.svg"
      alt={alt}
      width={size}
      height={size}
      className={styles.img}
    />
  )
}
