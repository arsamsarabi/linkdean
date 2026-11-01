import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { claimLinkFrom } from '@/services'
import { ShareInvitePanel } from './ShareInvitePanel'

vi.stubGlobal(
  'ResizeObserver',
  class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
)

function panel(claimLink: string | null) {
  const noop = vi.fn()
  return render(
    <MantineProvider env="test">
      <ShareInvitePanel
        description="Invite"
        email=""
        access="read"
        onEmailChange={noop}
        onAccessChange={noop}
        onInvite={noop}
        invitePending={false}
        claimLink={claimLink}
        sharesError={null}
        onRetryShares={noop}
        sharesLoading={false}
        activeShares={[]}
        pendingInvites={[]}
        onUpdateAccess={noop}
        onRevoke={noop}
        onUpdatePendingAccess={noop}
        onCancelPending={noop}
      />
    </MantineProvider>
  )
}

describe('share invite claim link', () => {
  afterEach(cleanup)

  it('builds a link only for a pending invite with a token', () => {
    expect(claimLinkFrom({ status: 'pending', invite: { claim_token: 'abc' } })).toBe(
      `${window.location.origin}/register?share=abc`
    )
    expect(claimLinkFrom({ status: 'pending', invite: {} })).toBeNull()
    expect(claimLinkFrom({ status: 'share' })).toBeNull()
  })

  it('shows the link after inviting someone with no account', () => {
    panel('http://app/register?share=abc')
    expect(screen.getByLabelText<HTMLInputElement>('Invite link').value).toBe('http://app/register?share=abc')
    expect(screen.getByRole('button', { name: 'Copy link' })).toBeTruthy()
  })

  it('hides the link otherwise', () => {
    panel(null)
    expect(screen.queryByLabelText('Invite link')).toBeNull()
  })
})
