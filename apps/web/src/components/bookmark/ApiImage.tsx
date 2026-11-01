import { useEffect, type ReactElement } from 'react'
import { Image, type ImageProps } from '@mantine/core'
import { useApiImage } from '@/hooks'

type Props = Omit<ImageProps, 'src' | 'onError'> & {
  src: string | null | undefined
  alt: string
  onError?: () => void
}

/** Mantine Image that can show authenticated `/api/` images. */
export function ApiImage({ src, alt, onError, ...props }: Props): ReactElement {
  const image = useApiImage(src)

  useEffect(() => {
    if (image.failed) onError?.()
    // oxlint-disable-next-line react-hooks/exhaustive-deps -- report each failure once
  }, [image.failed])

  return <Image src={image.src} alt={alt} onError={onError} {...props} />
}
