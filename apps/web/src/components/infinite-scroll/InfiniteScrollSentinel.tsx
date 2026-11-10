import { useEffect, type ReactElement } from 'react'
import { Center, Loader } from '@mantine/core'
import { useIntersection } from '@mantine/hooks'
import { useScrollRoot } from '@/hooks/render'

type Props = {
  loading: boolean
  onLoadMore: () => void
}

/** Render only while more pages exist. Asks for the next page when it scrolls near the viewport. */
export function InfiniteScrollSentinel({ loading, onLoadMore }: Props): ReactElement {
  const { ref, entry } = useIntersection<HTMLDivElement>({ root: useScrollRoot(), rootMargin: '400px' })
  const near = entry?.isIntersecting ?? false

  // Re-runs when a page lands (`loading` flips), so a still-visible sentinel keeps loading.
  useEffect(() => {
    if (near && !loading) onLoadMore()
  }, [near, loading, onLoadMore])

  return (
    <Center ref={ref} h={40} role="status" aria-live="polite">
      {loading ? <Loader size="sm" aria-label="Loading more bookmarks" /> : null}
    </Center>
  )
}
