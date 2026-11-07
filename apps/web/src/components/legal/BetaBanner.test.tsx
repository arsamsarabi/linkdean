import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { BetaBanner } from './BetaBanner'

function renderBanner(): void {
  render(
    <MantineProvider>
      <BetaBanner />
    </MantineProvider>
  )
}

describe('BetaBanner', () => {
  beforeEach(() => sessionStorage.clear())
  afterEach(cleanup)

  it('links to the terms and privacy pages', () => {
    renderBanner()
    expect(screen.getByRole('link', { name: 'Terms' }).getAttribute('href')).toBe('/terms')
    expect(screen.getByRole('link', { name: 'Privacy Policy' }).getAttribute('href')).toBe('/privacy')
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
