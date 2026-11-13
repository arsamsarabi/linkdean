import { type ReactNode } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, renderHook, waitFor } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useDashboardBoard } from './useDashboardBoard'

const summary = (id: string, is_default: boolean) => ({
  id,
  owner_user_id: 'u1',
  title: id,
  is_default,
  access: 'owner',
  shared_out: false,
  shared_with: [],
})

const boards = [summary('home', true)]

vi.mock('@tanstack/react-router', () => ({ useSearch: () => ({}) }))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))
vi.mock('@/hooks/bookmarks', () => ({ useBookmarks: () => ({ data: [] }) }))
vi.mock('@/api', () => ({
  api: {
    // The server is slow to list the new board, as after a real create.
    listDashboards: vi.fn(async () => {
      await new Promise((r) => setTimeout(r, 30))
      return [...boards]
    }),
    getDefaultDashboard: vi.fn(async () => ({ ...summary('home', true), sections: [] })),
    createDashboard: vi.fn(async () => {
      boards.push(summary('fresh', false))
      return { ...summary('fresh', false), sections: [] }
    }),
  },
}))

function wrapper({ children }: { children: ReactNode }) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return <QueryClientProvider client={qc}>{children}</QueryClientProvider>
}

describe('useDashboardBoard', () => {
  it('opens a new dashboard as soon as it is created', async () => {
    const { result } = renderHook(() => useDashboardBoard(), { wrapper })
    await waitFor(() => expect(result.current.activeId).toBe('home'))

    await act(() => result.current.onCreate(false))

    await waitFor(() => expect(result.current.list.data?.map((d) => d.id)).toContain('fresh'))
    expect(result.current.activeId).toBe('fresh')
  })
})
