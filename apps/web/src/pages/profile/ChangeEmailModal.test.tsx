import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ChangeEmailModal } from './ChangeEmailModal'
import { ProfileAboutCard } from './ProfileAboutCard'

type Body = { password: string; new_email: string }
type Change = {
  isPending: boolean
  isSuccess: boolean
  isError: boolean
  error: Error | null
  data?: { ok: boolean; hours: number }
  variables?: Body
  mutate: (body: Body) => void
}

const state: { change: Change } = {
  change: { isPending: false, isSuccess: false, isError: false, error: null, mutate: vi.fn() },
}

vi.mock('@/hooks', () => ({ useChangeEmail: () => state.change, useAvatarImage: () => undefined }))

function renderModal(): ReturnType<typeof render> {
  return render(
    <MantineProvider>
      <ChangeEmailModal currentEmail="ada@example.com" onClose={vi.fn()} />
    </MantineProvider>
  )
}

describe('ChangeEmailModal', () => {
  beforeEach(() => {
    state.change = { isPending: false, isSuccess: false, isError: false, error: null, mutate: vi.fn() }
  })
  afterEach(cleanup)

  it('submits the new address with the current password', async () => {
    renderModal()
    await userEvent.type(screen.getByLabelText('New email'), 'new@example.com')
    await userEvent.type(screen.getByLabelText('Current password'), 'hunter22hunter')
    await userEvent.click(screen.getByRole('button', { name: 'Send confirmation link' }))
    expect(state.change.mutate).toHaveBeenCalledWith({ password: 'hunter22hunter', new_email: 'new@example.com' })
  })

  it('says the email stays the same until the link is opened', () => {
    state.change = {
      ...state.change,
      isSuccess: true,
      data: { ok: true, hours: 24 },
      variables: { password: 'x', new_email: 'new@example.com' },
    }
    renderModal()
    expect(screen.getByText('new@example.com')).toBeTruthy()
    expect(screen.getByText(/Your email stays the same until you open it/)).toBeTruthy()
  })

  it('shows the API error', () => {
    state.change = { ...state.change, isError: true, error: new Error('email already registered') }
    renderModal()
    expect(screen.getByText('email already registered')).toBeTruthy()
  })
})

describe('ProfileAboutCard email', () => {
  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
  })
  // The bio Textarea autosizes with font and resize events jsdom does not provide.
  beforeEach(() => {
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe(): void {}
        unobserve(): void {}
        disconnect(): void {}
      }
    )
    if (!document.fonts) {
      Object.defineProperty(document, 'fonts', {
        configurable: true,
        value: { addEventListener: vi.fn(), removeEventListener: vi.fn() },
      })
    }
  })

  function renderCard(hasPassword: boolean, mailEnabled: boolean, onChangeEmail = vi.fn()): void {
    render(
      <MantineProvider>
        <ProfileAboutCard
          label="Ada"
          hasAvatar={false}
          avatarBust={0}
          displayName="Ada"
          bio=""
          timezone={null}
          email="ada@example.com"
          hasPassword={hasPassword}
          mailEnabled={mailEnabled}
          saving={false}
          resetRef={{ current: null }}
          onDisplayName={vi.fn()}
          onBio={vi.fn()}
          onTimezone={vi.fn()}
          onChangeEmail={onChangeEmail}
          onPickAvatar={vi.fn()}
          onRemoveAvatar={vi.fn()}
          onSave={vi.fn()}
        />
      </MantineProvider>
    )
  }

  it('shows the email read-only with a Change email button', async () => {
    const onChangeEmail = vi.fn()
    renderCard(true, true, onChangeEmail)
    expect((screen.getByLabelText('Email') as HTMLInputElement).readOnly).toBe(true)
    await userEvent.click(screen.getByRole('button', { name: 'Change email' }))
    expect(onChangeEmail).toHaveBeenCalled()
  })

  it('asks for an admin when mail is off', () => {
    renderCard(true, false)
    expect(screen.queryByRole('button', { name: 'Change email' })).toBeNull()
    expect(screen.getByText('Ask your admin to change it.')).toBeTruthy()
  })

  it('hides the button for accounts without a password', () => {
    renderCard(false, true)
    expect(screen.queryByRole('button', { name: 'Change email' })).toBeNull()
    expect(screen.getByText("Your identity provider manages this account's email.")).toBeTruthy()
  })
})
