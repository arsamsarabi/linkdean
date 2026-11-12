import type { ReactNode } from 'react';
import ThemedImage from '@theme/ThemedImage';

type Props = {
  /** File stem under /img/screenshots, without the -light/-dark suffix. */
  name: string;
  alt: string;
  /** Phone screenshots render narrower. */
  phone?: boolean;
};

/** A product screenshot that follows the docs theme: light image in light mode, dark in dark. */
export default function Shot({ name, alt, phone = false }: Props): ReactNode {
  const [width, height] = phone ? [390, 844] : [1600, 1000];
  return (
    <figure style={{ margin: '1.5rem 0', textAlign: 'center' }}>
      <ThemedImage
        alt={alt}
        width={width}
        height={height}
        loading="lazy"
        style={{
          width: '100%',
          maxWidth: phone ? 300 : '100%',
          height: 'auto',
          borderRadius: 12,
          border: '1px solid var(--ifm-color-emphasis-300)',
          boxShadow: 'var(--ifm-global-shadow-md)',
        }}
        sources={{
          light: `/img/screenshots/${name}-light.webp`,
          dark: `/img/screenshots/${name}-dark.webp`,
        }}
      />
    </figure>
  );
}
