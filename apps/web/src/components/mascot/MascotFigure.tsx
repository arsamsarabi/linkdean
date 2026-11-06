import type { ReactElement } from 'react'
import type { MascotSrc } from './mascotAssets'
import styles from './MascotFigure.module.css'

type Props = {
  src: MascotSrc | string
  /** CSS display width in px. */
  width: number
  className?: string
}

/** Decorative Dean with a soft light plate so the dark robe reads on dark theme. */
export function MascotFigure({ src, width, className }: Props): ReactElement {
  return (
    <div className={[styles.plate, className].filter(Boolean).join(' ')} style={{ width: width + 48 }}>
      <img className={styles.img} src={src} width={width} alt="" loading="lazy" decoding="async" />
    </div>
  )
}
