import type { ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '@/api'
import { ProfileBlocksCard } from './ProfileBlocksCard'

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return {
    ...actual,
    api: {
      ...actual.api,
      myBlocked: vi.fn(),
      myMuted: vi.fn(),
      unblockUser: vi.fn(),
      unmuteUser: vi.fn(),
    },
  }
})

const mocked = vi.mocked(api)
const bo = { id: 'u-bo', display_name: 'Bo', created_at: '2026-11-01T00:00:00Z' }
const cy = { id: 'u-cy', display_name: 'Cy', created_at: '2026-11-01T00:00:00Z' }

function wrap(ui: ReactElement): ReactElement {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return (
    <QueryClientProvider client={client}>
      <MantineProvider>{ui}</MantineProvider>
    </QueryClientProvider>
  )
}

describe('ProfileBlocksCard', () => {
  afterEach(cleanup)

  beforeEach((): void => {
    vi.resetAllMocks()
    mocked.myBlocked.mockResolvedValue({ users: [] })
    mocked.myMuted.mockResolvedValue({ users: [] })
  })

  it('shows loading until lists arrive', async (): Promise<void> => {
    let resolve: (v: { users: never[] }) => void = () => undefined
    mocked.myBlocked.mockReturnValue(new Promise((r) => (resolve = r)))
    render(wrap(<ProfileBlocksCard />))
    expect(screen.getAllByText('Loading…').length).toBeGreaterThan(0)
    resolve({ users: [] })
    expect(await screen.findByText(/nobody blocked/i)).toBeInTheDocument()
  })

  it('shows empty states', async (): Promise<void> => {
    render(wrap(<ProfileBlocksCard />))
    expect(await screen.findByText(/nobody blocked/i)).toBeInTheDocument()
    expect(await screen.findByText('Nobody muted.')).toBeInTheDocument()
  })

  it('shows an error and retries', async (): Promise<void> => {
    mocked.myBlocked.mockRejectedValueOnce(new Error('network down'))
    const user = userEvent.setup()
    render(wrap(<ProfileBlocksCard />))
    expect(await screen.findByText('network down')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /retry/i }))
    expect(await screen.findByText(/nobody blocked/i)).toBeInTheDocument()
  })

  it('unblocks the chosen user and refetches', async (): Promise<void> => {
    mocked.myBlocked.mockResolvedValueOnce({ users: [bo, cy] })
    mocked.unblockUser.mockResolvedValue({ block: { blocked: false } })
    const user = userEvent.setup()
    render(wrap(<ProfileBlocksCard />))
    await user.click(await screen.findByRole('button', { name: 'Unblock Cy' }))
    expect(mocked.unblockUser).toHaveBeenCalledWith('u-cy')
    await waitFor(() => expect(mocked.myBlocked).toHaveBeenCalledTimes(2))
    expect(await screen.findByText(/nobody blocked/i)).toBeInTheDocument()
  })

  it('unmutes the chosen user', async (): Promise<void> => {
    mocked.myMuted.mockResolvedValueOnce({ users: [bo] })
    mocked.unmuteUser.mockResolvedValue({ mute: { muted: false } })
    const user = userEvent.setup()
    render(wrap(<ProfileBlocksCard />))
    await user.click(await screen.findByRole('button', { name: 'Unmute Bo' }))
    expect(mocked.unmuteUser).toHaveBeenCalledWith('u-bo')
  })
})
