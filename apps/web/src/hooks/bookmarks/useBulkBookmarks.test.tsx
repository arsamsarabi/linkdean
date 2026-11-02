import type { ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { api } from '@/api'
import { useBulkAddCategories, useBulkTagBookmarks } from './useBulkBookmarks'

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
}

describe('bulk add', () => {
  afterEach(() => vi.restoreAllMocks())

  it('asks the server to add, without sending a snapshot of current values', async () => {
    const update = vi.spyOn(api, 'updateBookmark').mockResolvedValue({} as never)
    const tags = renderHook(() => useBulkTagBookmarks(), { wrapper })
    await tags.result.current.mutateAsync({ ids: ['a', 'b'], tags: [' new ', ''], mode: 'add' })
    expect(update).toHaveBeenCalledWith('a', { add_tags: ['new'] })
    expect(update).toHaveBeenCalledWith('b', { add_tags: ['new'] })

    const folders = renderHook(() => useBulkAddCategories(), { wrapper })
    await folders.result.current.mutateAsync({ ids: ['a'], category_ids: ['f'] })
    expect(update).toHaveBeenCalledWith('a', { add_category_ids: ['f'] })
  })

  it('still replaces tags outright in replace mode', async () => {
    const update = vi.spyOn(api, 'updateBookmark').mockResolvedValue({} as never)
    const tags = renderHook(() => useBulkTagBookmarks(), { wrapper })
    await tags.result.current.mutateAsync({ ids: ['a'], tags: ['x'], mode: 'replace' })
    expect(update).toHaveBeenCalledWith('a', { tags: ['x'] })
  })
})
