import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ProfileSsoCard } from './ProfileSsoCard'

type Props = Parameters<typeof ProfileSsoCard>[0]

function renderCard(props: Partial<Props> = {}): Props {
  const all: Props = {
    providers: [
      { id: 'oidc', label: 'Authentik' },
      { id: 'github', label: 'GitHub' },
    ],
    connected: false,
    provider: null,
    hasPassword: true,
    error: null,
    onDismissError: vi.fn(),
    onDisconnect: vi.fn(),
    ...props,
  }
  render(
    <MantineProvider>
      <ProfileSsoCard {...all} />
    </MantineProvider>
  )
  return all
}

describe('ProfileSsoCard', () => {
  afterEach(cleanup)

  it('offers Connect for each provider when nothing is attached', () => {
    renderCard()
    expect(screen.getByRole('link', { name: 'Connect Authentik' }).getAttribute('href')).toBe(
      '/api/auth/oidc/start?mode=connect&provider=oidc'
    )
    expect(screen.getByRole('link', { name: 'Connect GitHub' }).getAttribute('href')).toBe(
      '/api/auth/oidc/start?mode=connect&provider=github'
    )
    expect(screen.queryByRole('button', { name: 'Disconnect' })).toBeNull()
  })

  it('shows the provider and Disconnect when connected', async () => {
    const props = renderCard({ connected: true, provider: 'id.example.com' })
    expect(screen.getByText('Connected')).toBeTruthy()
    expect(screen.getByText('id.example.com')).toBeTruthy()
    expect(screen.queryByRole('link', { name: /^Connect/ })).toBeNull()
    await userEvent.click(screen.getByRole('button', { name: 'Disconnect' }))
    expect(props.onDisconnect).toHaveBeenCalled()
  })

  it('hides Disconnect for an account without a password', () => {
    renderCard({ connected: true, provider: 'id.example.com', hasPassword: false })
    expect(screen.queryByRole('button', { name: 'Disconnect' })).toBeNull()
    expect(screen.getByText(/only way to sign in/)).toBeTruthy()
  })

  it('shows why the last Connect failed', () => {
    renderCard({ error: 'This sign-in is already connected to another account.' })
    expect(screen.getByText('This sign-in is already connected to another account.')).toBeTruthy()
  })
})
