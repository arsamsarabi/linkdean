import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { ConnectExtensionPage } from './ConnectExtensionPage'

const search = vi.hoisted(() => ({ value: {} as Record<string, string | undefined> }))
const authorize = vi.hoisted(() => vi.fn())

vi.mock('@tanstack/react-router', () => ({ useSearch: () => search.value }))
vi.mock('@/api', () => ({ api: { authorizeExtension: authorize } }))
vi.mock('@/hooks', () => ({ useMe: () => ({ data: { user: { email: 'sam@example.com' } } }) }))

const redirect = `https://${'a'.repeat(32)}.chromiumapp.org/`
const challenge = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM'

function renderPage(): void {
  render(
    <MantineProvider env="test">
      <ConnectExtensionPage />
    </MantineProvider>
  )
}

describe('ConnectExtensionPage', () => {
  const assign = vi.fn()
  beforeEach(() => {
    vi.stubGlobal('location', { ...window.location, assign })
    search.value = { redirect_uri: redirect, code_challenge: challenge, state: 's1' }
  })
  afterEach(() => {
    cleanup()
    vi.unstubAllGlobals()
    authorize.mockReset()
    assign.mockReset()
  })

  it('names the account and sends the window back to the extension on Allow', async () => {
    authorize.mockResolvedValue({ redirect: `${redirect}?code=c1&state=s1` })
    renderPage()
    expect(screen.getByText('sam@example.com')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Allow' }))
    expect(authorize).toHaveBeenCalledWith({
      redirect_uri: redirect,
      code_challenge: challenge,
      code_challenge_method: 'S256',
      state: 's1',
    })
    expect(assign).toHaveBeenCalledWith(`${redirect}?code=c1&state=s1`)
  })

  it('shows why the server refused, and stays on the page', async () => {
    authorize.mockRejectedValue(new Error('this sign-in did not come from the Linkdean extension'))
    renderPage()
    await userEvent.click(screen.getByRole('button', { name: 'Allow' }))
    expect((await screen.findByRole('alert')).textContent).toContain('did not come from the Linkdean extension')
    expect(assign).not.toHaveBeenCalled()
  })

  it('asks to start from the extension when the request is incomplete', () => {
    search.value = {}
    renderPage()
    expect(screen.getByText('Start from the extension')).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
  })

  it('Cancel sends nothing back', async () => {
    const close = vi.spyOn(window, 'close').mockImplementation(() => undefined)
    renderPage()
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(authorize).not.toHaveBeenCalled()
    expect(screen.getByText(/Cancelled/)).toBeTruthy()
    close.mockRestore()
  })
})
