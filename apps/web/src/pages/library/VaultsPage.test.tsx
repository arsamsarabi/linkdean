import { cleanup, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { VaultsPage } from './VaultsPage'

const me = vi.hoisted(() => ({ data: undefined as { vault_unlocked: boolean } | undefined }))
const navigate = vi.hoisted(() => vi.fn())
const show = vi.hoisted(() => vi.fn())

vi.mock('@/hooks', () => ({ useMe: () => ({ data: me.data }) }))
vi.mock('@mantine/notifications', () => ({ notifications: { show } }))
vi.mock('@tanstack/react-router', () => ({ useNavigate: () => navigate }))
vi.mock('./LibraryPage', () => ({ LibraryPage: ({ scope }: { scope: string }) => <div>page {scope}</div> }))

describe('VaultsPage', () => {
  afterEach(() => {
    cleanup()
    navigate.mockReset()
    show.mockReset()
  })

  it('renders nothing until the user has loaded', () => {
    me.data = undefined
    render(<VaultsPage />)
    expect(screen.queryByText('page vault')).toBeNull()
    expect(navigate).not.toHaveBeenCalled()
  })

  it('shows the vault scope while a vault is open', () => {
    me.data = { vault_unlocked: true }
    render(<VaultsPage />)
    expect(screen.getByText('page vault')).toBeTruthy()
  })

  it('leaves for the library when no vault is open', async () => {
    me.data = { vault_unlocked: false }
    render(<VaultsPage />)
    expect(screen.queryByText('page vault')).toBeNull()
    await waitFor(() => expect(navigate).toHaveBeenCalledWith({ to: '/library', replace: true }))
    expect(show).toHaveBeenCalledWith({ message: 'No vaults are open' })
  })
})
