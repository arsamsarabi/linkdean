import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { BetaBanner } from './BetaBanner'

const bootstrap: { data: Record<string, string> | undefined } = { data: undefined }

vi.mock('@/hooks', () => ({ useBootstrap: () => bootstrap }))

function renderBanner(): void {
  render(
    <MantineProvider>
      <BetaBanner />
    </MantineProvider>
  )
}

describe('BetaBanner', () => {
  beforeEach(() => {
    sessionStorage.clear()
    bootstrap.data = { terms_url: 'https://example.com/terms', privacy_url: 'https://example.com/privacy' }
  })
  afterEach(cleanup)

  it("links to the server's terms and privacy pages", () => {
    renderBanner()
    expect(screen.getByRole('link', { name: 'Terms' }).getAttribute('href')).toBe('https://example.com/terms')
    expect(screen.getByRole('link', { name: 'Privacy Policy' }).getAttribute('href')).toBe(
      'https://example.com/privacy'
    )
  })

  it('shows nothing on a server without its own terms', () => {
    bootstrap.data = {}
    renderBanner()
    expect(screen.queryByText(/public beta/)).toBeNull()
  })

  it('stays dismissed for the session', async () => {
    renderBanner()
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss beta notice' }))
    expect(screen.queryByText(/public beta/)).toBeNull()
    cleanup()
    renderBanner()
    expect(screen.queryByText(/public beta/)).toBeNull()
  })
})
