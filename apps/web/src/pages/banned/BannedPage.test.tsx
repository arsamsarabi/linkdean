import type { ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api'
import { BannedPage } from './BannedPage'

const me = vi.hoisted(() => vi.fn())
const navigate = vi.hoisted(() => vi.fn())

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return { ...actual, api: { me } }
})
vi.mock('@/hooks', () => ({ useLogout: () => ({ mutateAsync: vi.fn(), isPending: false }) }))
vi.mock('@tanstack/react-router', () => ({ useNavigate: () => navigate }))

function ui(): ReactElement {
  return (
    <MantineProvider>
      <QueryClientProvider client={new QueryClient()}>
        <BannedPage />
      </QueryClientProvider>
    </MantineProvider>
  )
}

const banned = (ends_at: string | null) =>
  new ApiError('banned', 403, { code: 'banned', ban: { reason: 'Spam', ends_at } })

describe('BannedPage', () => {
  afterEach(() => {
    cleanup()
    navigate.mockReset()
    me.mockReset()
  })

  it('shows the reason and says a ban is permanent', async () => {
    me.mockRejectedValue(banned(null))
    render(ui())
    expect(await screen.findByText('Spam')).toBeTruthy()
    expect(screen.getByText('This ban is permanent.')).toBeTruthy()
  })

  it('shows when a ban ends', async () => {
    me.mockRejectedValue(banned('2030-01-02T12:00:00Z'))
    render(ui())
    expect(await screen.findByText(/This ban ends on/)).toBeTruthy()
  })

  it('goes to sign in when there is no session', async () => {
    me.mockRejectedValue(new ApiError('unauthorized', 401, {}))
    render(ui())
    await waitFor(() => expect(navigate).toHaveBeenCalledWith({ to: '/login', replace: true }))
  })

  it('returns to the app once the ban has ended', async () => {
    me.mockResolvedValue({})
    render(ui())
    await waitFor(() => expect(navigate).toHaveBeenCalledWith({ to: '/', replace: true }))
  })
})
