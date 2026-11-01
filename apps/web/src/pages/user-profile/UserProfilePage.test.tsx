import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { api } from '@/api'
import { UserProfileView } from './UserProfilePage'

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return { ...actual, api: { ...actual.api, userProfile: vi.fn(), userBookmarks: vi.fn() } }
})

describe('UserProfileView', () => {
  afterEach(cleanup)

  it('shows the unavailable state on a denied profile', async () => {
    vi.mocked(api.userProfile).mockRejectedValue(new Error('not found'))
    render(
      <MantineProvider>
        <QueryClientProvider client={new QueryClient()}>
          <UserProfileView userId="u1" onEdit={vi.fn()} />
        </QueryClientProvider>
      </MantineProvider>
    )
    expect(await screen.findByText('Profile not available')).toBeTruthy()
  })
})
