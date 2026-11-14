import { type ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ApiError } from '@/api'
import { UnsubscribePage } from './UnsubscribePage'

const api = vi.hoisted(() => ({
  checkUnsubscribeLink: vi.fn(),
  unsubscribe: vi.fn(),
}))

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return { ...actual, api }
})

function ui(): ReactElement {
  return (
    <MantineProvider>
      <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
        <UnsubscribePage />
      </QueryClientProvider>
    </MantineProvider>
  )
}

const QUERY = 'u=00000000-0000-0000-0000-000000000001&s=social_like&t=signed'

describe('UnsubscribePage', () => {
  beforeEach(() => {
    window.history.replaceState(null, '', `/unsubscribe?${QUERY}&utm=x`)
    api.checkUnsubscribeLink.mockReset()
    api.unsubscribe.mockReset()
  })
  afterEach(cleanup)

  it('asks first, then unsubscribes with the signed link', async () => {
    api.checkUnsubscribeLink.mockResolvedValue({ scope: 'social_like', label: 'Likes' })
    api.unsubscribe.mockResolvedValue({ scope: 'social_like', label: 'Likes', unsubscribed: ['social_like'] })
    render(ui())
    expect(await screen.findByText('Stop email for Likes?')).toBeTruthy()
    expect(api.checkUnsubscribeLink).toHaveBeenCalledWith(QUERY)
    expect(api.unsubscribe).not.toHaveBeenCalled()
    await userEvent.click(screen.getByRole('button', { name: 'Unsubscribe' }))
    expect(api.unsubscribe).toHaveBeenCalledWith(QUERY)
    expect(await screen.findByText('Unsubscribed')).toBeTruthy()
    expect(screen.getByText('You will not get email for Likes any more.')).toBeTruthy()
  })

  it('names the digest', async () => {
    api.checkUnsubscribeLink.mockResolvedValue({ scope: 'digest', label: 'Daily digest' })
    render(ui())
    expect(await screen.findByText('Stop the daily digest?')).toBeTruthy()
  })

  it('says when the link is not valid', async () => {
    api.checkUnsubscribeLink.mockRejectedValue(new ApiError('not found', 404, {}))
    render(ui())
    expect(await screen.findByText('Link not valid')).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Unsubscribe' })).toBeNull()
  })
})
