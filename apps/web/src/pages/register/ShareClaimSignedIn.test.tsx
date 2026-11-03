import { StrictMode } from 'react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api'
import { ShareClaimSignedIn } from './ShareClaimSignedIn'

const claim = vi.hoisted(() => vi.fn())

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return { ...actual, api: { claimShareInvite: claim } }
})
vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return { ...actual, useNavigate: () => vi.fn() }
})

function renderClaim(token: string): void {
  render(
    <StrictMode>
      <MantineProvider>
        <QueryClientProvider client={new QueryClient()}>
          <ShareClaimSignedIn token={token} />
        </QueryClientProvider>
      </MantineProvider>
    </StrictMode>
  )
}

describe('ShareClaimSignedIn', () => {
  afterEach(() => {
    cleanup()
    claim.mockReset()
  })

  it('claims the invite once and points to Shared', async () => {
    claim.mockResolvedValue({ ok: true })
    renderClaim('share_a')
    expect(await screen.findByText('Invite added')).toBeTruthy()
    expect(claim).toHaveBeenCalledTimes(1)
    expect(claim).toHaveBeenCalledWith('share_a')
  })

  it('shows why a claim failed', async () => {
    const msg = 'this invite link was already used, or was sent to a different email'
    claim.mockRejectedValue(new ApiError(msg, 400, {}))
    renderClaim('share_b')
    expect(await screen.findByText(msg)).toBeTruthy()
  })
})
