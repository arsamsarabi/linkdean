import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { TERMS_VERSION } from '@/lib'
import { TermsUpdateModal } from './TermsUpdateModal'

const accept = { isPending: false, isError: false, error: null, mutate: vi.fn() }

vi.mock('@/hooks', () => ({ useAcceptTerms: () => accept }))

function renderModal(outdated: boolean, onSignOut = vi.fn()): void {
  render(
    <MantineProvider>
      <TermsUpdateModal outdated={outdated} onSignOut={onSignOut} />
    </MantineProvider>
  )
}

describe('TermsUpdateModal', () => {
  afterEach(() => {
    cleanup()
    accept.mutate.mockClear()
  })

  it('shows nothing while the account is on the current terms', () => {
    renderModal(false)
    expect(screen.queryByText('Our terms changed')).toBeNull()
  })

  it('accepts the current version, links the policies and cannot be closed', async () => {
    renderModal(true)
    expect(screen.getByText('Our terms changed')).toBeTruthy()
    expect(screen.queryByRole('button', { name: /close/i })).toBeNull()
    expect(screen.getByRole('link', { name: 'Terms of Service' }).getAttribute('href')).toBe('/terms')
    expect(screen.getByRole('link', { name: 'Privacy Policy' }).getAttribute('href')).toBe('/privacy')
    await userEvent.click(screen.getByRole('button', { name: 'Accept and continue' }))
    expect(accept.mutate).toHaveBeenCalledWith(TERMS_VERSION)
  })

  it('offers to sign out instead', async () => {
    const onSignOut = vi.fn()
    renderModal(true, onSignOut)
    await userEvent.click(screen.getByRole('button', { name: 'Sign out' }))
    expect(onSignOut).toHaveBeenCalled()
  })
})
