import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { suggestedPasskeyName } from '@/lib'
import { ProfilePasskeysCard } from './ProfilePasskeysCard'

const add = vi.hoisted(() => ({
  isPending: false,
  error: null as Error | null,
  mutate: vi.fn(),
  reset: vi.fn(),
}))
const passkeys = vi.hoisted(() => ({
  list: [] as { id: string; name: string; created_at: string; last_used_at: string | null }[],
}))

vi.mock('@/hooks', () => ({
  usePasskeys: () => ({ data: { passkeys: passkeys.list }, isSuccess: true }),
  useAddPasskey: () => add,
  useRenamePasskey: () => ({ isPending: false, mutate: vi.fn() }),
  useDeletePasskey: () => ({ isPending: false, mutate: vi.fn() }),
}))
vi.mock('@/components', () => ({ ConfirmModal: () => null }))

function renderCard(hasPassword: boolean, reauthed = false): void {
  render(
    <MantineProvider>
      <ProfilePasskeysCard hasPassword={hasPassword} reauthed={reauthed} />
    </MantineProvider>
  )
}

describe('ProfilePasskeysCard', () => {
  beforeEach(() => {
    vi.stubGlobal('PublicKeyCredential', function PublicKeyCredential() {})
    add.mutate.mockClear()
    add.error = null
    passkeys.list = []
  })
  afterEach(() => {
    vi.unstubAllGlobals()
    cleanup()
  })

  it('lists passkeys with when they were added and used', () => {
    passkeys.list = [{ id: '1', name: 'Mac', created_at: '2026-11-01T10:00:00Z', last_used_at: null }]
    renderCard(true)
    expect(screen.getByText('Mac')).toBeTruthy()
    expect(screen.getByText(/never used/)).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Rename Mac' })).toBeTruthy()
    expect(screen.getByRole('button', { name: 'Remove Mac' })).toBeTruthy()
  })

  it('asks a password account for its password before adding', async () => {
    renderCard(true)
    await userEvent.click(screen.getByRole('button', { name: 'Add passkey' }))
    const pw = await screen.findByLabelText('Current password')
    const dialog = screen.getByRole('dialog')
    const confirm = within(dialog).getByRole('button', { name: 'Add passkey' }) as HTMLButtonElement
    expect(confirm.disabled).toBe(true)
    await userEvent.type(pw, 'hunter22hunter')
    await userEvent.click(confirm)
    expect(add.mutate).toHaveBeenCalledWith({ name: expect.any(String), password: 'hunter22hunter' }, expect.anything())
  })

  it('sends an SSO-only account to sign in again, back to this page', async () => {
    renderCard(false)
    await userEvent.click(screen.getByRole('button', { name: 'Add passkey' }))
    expect((await screen.findByRole('link', { name: 'Sign in again' })).getAttribute('href')).toBe(
      '/api/auth/oidc/start?mode=reauth&return=profile'
    )
  })

  it('opens the add dialog after signing in again, without a password box', () => {
    renderCard(false, true)
    expect(screen.queryByLabelText('Current password')).toBeNull()
    expect(screen.getByLabelText(/Name/)).toBeTruthy()
  })

  it('suggests a name from the device', () => {
    expect(suggestedPasskeyName('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)')).toBe('Mac')
    expect(suggestedPasskeyName('Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)')).toBe('iPhone')
    expect(suggestedPasskeyName('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe('Windows')
    expect(suggestedPasskeyName('curl/8')).toBe('This device')
  })
})
