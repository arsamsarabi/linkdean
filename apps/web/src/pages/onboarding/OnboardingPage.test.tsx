import type { ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '@/api'
import { markSignedIn } from '@/lib'
import type { MeResponse } from '@/schemas'
import { OnboardingPage } from './OnboardingPage'

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return {
    ...actual,
    api: {
      ...actual.api,
      bootstrap: vi.fn(),
      me: vi.fn(),
      register: vi.fn(),
      createVault: vi.fn(),
    },
  }
})

const navigate = vi.hoisted(() => vi.fn())

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return { ...actual, useNavigate: () => navigate }
})

// jsdom has no ResizeObserver; the import step's Mantine Select needs one.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
)

const mocked = vi.mocked(api)
const user = { is_admin: true, onboarding_completed: false }

function wrap(ui: ReactElement): ReactElement {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  return (
    <QueryClientProvider client={client}>
      <MantineProvider>{ui}</MantineProvider>
    </QueryClientProvider>
  )
}

async function createAdmin(person: ReturnType<typeof userEvent.setup>): Promise<void> {
  await person.type(await screen.findByRole('textbox', { name: /email/i }), 'ada@example.com')
  await person.type(screen.getByLabelText(/^password/i), 'password-long')
  await person.type(screen.getByLabelText(/^confirm password/i), 'password-long')
  await person.click(screen.getByRole('button', { name: /create admin account/i }))
}

describe('OnboardingPage', () => {
  afterEach(cleanup)

  beforeEach((): void => {
    vi.resetAllMocks()
    markSignedIn(false)
    mocked.bootstrap.mockResolvedValue({
      needs_bootstrap: true,
      bootstrap_token_required: false,
      oidc_enabled: false,
      registration_open: true,
      mail_enabled: false,
    })
    mocked.me.mockResolvedValue({ user, settings: {} } as unknown as MeResponse)
    mocked.register.mockResolvedValue({ token: 'tok', user } as never)
  })

  it('moves on to the vault step after creating the admin', async (): Promise<void> => {
    const person = userEvent.setup()
    render(wrap(<OnboardingPage />))

    await person.click(await screen.findByRole('button', { name: /get started/i }))
    await createAdmin(person)

    expect(await screen.findByLabelText(/^vault passphrase/i)).toBeInTheDocument()
    await vi.waitFor(() => expect(mocked.me).toHaveBeenCalled())
    await new Promise((resolve) => setTimeout(resolve, 50))
    expect(screen.queryByRole('button', { name: /get started/i })).not.toBeInTheDocument()
    expect(screen.getByLabelText(/^vault passphrase/i)).toBeInTheDocument()
  })

  it('asks for the setup token when the server has one', async (): Promise<void> => {
    mocked.bootstrap.mockResolvedValue({
      needs_bootstrap: true,
      bootstrap_token_required: true,
      oidc_enabled: false,
      registration_open: true,
      mail_enabled: false,
    })
    const person = userEvent.setup()
    render(wrap(<OnboardingPage />))

    await person.click(await screen.findByRole('button', { name: /get started/i }))
    await person.type(screen.getByLabelText(/^setup token/i), ' s3cret ')
    await createAdmin(person)

    await vi.waitFor(() =>
      expect(mocked.register).toHaveBeenCalledWith(expect.objectContaining({ bootstrap_token: 's3cret' }))
    )
  })

  it('shows the setup steps for the first account', async (): Promise<void> => {
    render(wrap(<OnboardingPage />))

    expect(await screen.findByRole('button', { name: /get started/i })).toBeInTheDocument()
    expect(screen.getByText('Admin')).toBeInTheDocument()
    expect(screen.getByText(/first account becomes/i)).toBeInTheDocument()
  })

  it('shows members only the optional steps', async (): Promise<void> => {
    markSignedIn(true)
    mocked.bootstrap.mockResolvedValue({
      needs_bootstrap: false,
      bootstrap_token_required: false,
      oidc_enabled: false,
      registration_open: true,
      mail_enabled: false,
    })
    mocked.me.mockResolvedValue({
      user: { is_admin: false, onboarding_completed: false },
      settings: {},
    } as unknown as MeResponse)
    const person = userEvent.setup()
    render(wrap(<OnboardingPage />))

    expect(await screen.findByLabelText(/^vault passphrase/i)).toBeInTheDocument()
    expect(screen.queryByText(/admin/i)).not.toBeInTheDocument()
    expect(screen.queryByText('Welcome')).not.toBeInTheDocument()

    await person.click(screen.getByRole('button', { name: /skip for now/i }))
    expect(await screen.findByRole('button', { name: /skip & finish/i })).toBeInTheDocument()
  })

  it('stays on import after enabling the vault refreshes the user', async (): Promise<void> => {
    mocked.createVault.mockResolvedValue({
      vault: { id: 'v1', name: 'Vault', unlocked: true, has_recovery_key: true, in_global: false },
      recovery_key: 'ABCD-EFGH',
    })
    const person = userEvent.setup()
    render(wrap(<OnboardingPage />))

    await person.click(await screen.findByRole('button', { name: /get started/i }))
    await createAdmin(person)
    await person.type(await screen.findByLabelText(/^vault passphrase/i), 'vault-pass-long')
    await person.type(screen.getByLabelText(/^confirm passphrase/i), 'vault-pass-long')
    mocked.me.mockResolvedValue({ user: { ...user, has_vault: true }, settings: {} } as unknown as MeResponse)
    await person.click(screen.getByRole('button', { name: /enable vault/i }))

    await person.click(await screen.findByLabelText('I saved this recovery key'))
    await person.click(screen.getByRole('button', { name: 'Done' }))

    expect(await screen.findByRole('button', { name: /skip & finish/i })).toBeInTheDocument()
    await new Promise((resolve) => setTimeout(resolve, 50))
    expect(screen.getByRole('button', { name: /skip & finish/i })).toBeInTheDocument()
  })
})
