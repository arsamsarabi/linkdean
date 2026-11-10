import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Bookmark } from '@/schemas'
import { BrokenLinksPage } from './BrokenLinksPage'

const removeBookmark = vi.fn(async (_id: string) => undefined)
const broken = [{ id: 'b1', title: 'Dead page', url: 'https://gone.example.com', link_check_status: 'broken' }]

vi.mock('@/hooks', () => ({
  brokenLinksQueryKey: ['broken-links'],
  useBrokenLinks: () => ({ data: broken as unknown as Bookmark[], isLoading: false, isError: false }),
  useRecheckBookmarkLink: () => ({ isPending: false, mutateAsync: vi.fn() }),
  useDeleteBookmark: () => ({ isPending: false, variables: undefined, mutateAsync: removeBookmark }),
}))

describe('BrokenLinksPage', () => {
  afterEach(cleanup)

  it('removes a broken bookmark', async () => {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MantineProvider>
          <BrokenLinksPage />
        </MantineProvider>
      </QueryClientProvider>
    )
    await userEvent.click(await screen.findByRole('button', { name: 'Remove bookmark Dead page' }))
    expect(removeBookmark).toHaveBeenCalledWith('b1')
  })
})
