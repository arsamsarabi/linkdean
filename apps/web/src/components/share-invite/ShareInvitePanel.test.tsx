import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { createCategoryShareOutcomeSchema, createDashboardShareOutcomeSchema } from '@/schemas'
import { claimLinkFrom, inviteEmailStatus, inviteToast, type InviteEmailStatus } from '@/services'
import { ShareInvitePanel } from './ShareInvitePanel'

vi.stubGlobal(
  'ResizeObserver',
  class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
)

function panel(claimLink: string | null, inviteEmail: InviteEmailStatus = null) {
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
        inviteEmail={inviteEmail}
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

describe('share invite email outcome', () => {
  afterEach(cleanup)

  it('reads the email outcome from the share response', () => {
    expect(inviteEmailStatus({ email_sent: true })).toBe('sent')
    expect(inviteEmailStatus({ email_sent: false, email_limited: true })).toBe('limited')
    expect(inviteEmailStatus({ email_sent: false })).toBeNull()
    expect(inviteEmailStatus({})).toBeNull()
  })

  it('keeps the email fields when parsing share responses', () => {
    const invite = {
      id: 'p1',
      subject_type: 'category',
      subject_id: 'c1',
      invitee_email: 'new@example.com',
      access: 'read',
      invited_by_user_id: 'u1',
    }
    const folder = createCategoryShareOutcomeSchema.parse({ status: 'pending', invite, email_sent: true })
    expect(inviteEmailStatus(folder)).toBe('sent')
    const board = createDashboardShareOutcomeSchema.parse({
      status: 'pending',
      invite: { ...invite, subject_type: 'dashboard' },
      email_sent: false,
      email_limited: true,
    })
    expect(inviteEmailStatus(board)).toBe('limited')
  })

  it('keeps the old toasts when nothing was emailed', () => {
    expect(inviteToast('http://app/register?share=abc', null)).toBe('Invite created. Send them the link.')
    expect(inviteToast(null, null)).toBe('Invite sent')
    expect(inviteToast(null, 'sent')).toBe('Invite created.')
  })

  it('says the invite was emailed', () => {
    panel('http://app/register?share=abc', 'sent')
    expect(screen.getByText('We emailed them an invite.')).toBeTruthy()
    expect(screen.queryByText(/too many invites/)).toBeNull()
  })

  it('warns when the hourly limit stopped the email', () => {
    panel('http://app/register?share=abc', 'limited')
    expect(
      screen.getByText(
        'The invite was saved, but no email was sent: too many invites in the last hour. Copy the link instead.'
      )
    ).toBeTruthy()
    expect(screen.queryByText('We emailed them an invite.')).toBeNull()
  })

  it('does not offer a link to copy when there is none', () => {
    panel(null, 'limited')
    expect(screen.getByText(/too many invites in the last hour\. They still see it in Linkdean\./)).toBeTruthy()
  })

  it('says nothing about email when none was sent', () => {
    panel(null)
    expect(screen.queryByText('We emailed them an invite.')).toBeNull()
    expect(screen.queryByText(/too many invites/)).toBeNull()
  })
})
