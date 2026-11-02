import { MantineProvider } from '@mantine/core'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { RegisterPage } from './RegisterPage'

vi.mock('@/api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/api')>()
  return {
    ...actual,
    api: {
      ...actual.api,
      bootstrap: vi.fn().mockResolvedValue({
        needs_bootstrap: false,
        oidc_enabled: false,
        registration_open: false,
      }),
    },
  }
})

vi.mock('@tanstack/react-router', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@tanstack/react-router')>()
  return { ...actual, useNavigate: () => vi.fn(), useSearch: () => ({}) }
})

describe('RegisterPage invite gate', () => {
  afterEach(cleanup)

  it('keeps the gate and its focus while a code is typed', async () => {
    render(
      <QueryClientProvider client={new QueryClient()}>
        <MantineProvider>
          <RegisterPage />
        </MantineProvider>
      </QueryClientProvider>
    )
    const input = await screen.findByLabelText('Invite code', {}, { timeout: 5000 })
    await userEvent.type(input, 'ldi_abc')
    expect(screen.getByText('Registration closed')).toBeTruthy()
    expect(document.activeElement).toBe(screen.getByLabelText('Invite code'))
    expect((screen.getByLabelText('Invite code') as HTMLInputElement).value).toBe('ldi_abc')
  })
})
