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

const origin = `chrome-extension://${'a'.repeat(32)}/`
const challenge = 'E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM'

function renderPage(): void {
  render(
    <MantineProvider env="test">
      <ConnectExtensionPage />
    </MantineProvider>
  )
}

describe('ConnectExtensionPage', () => {
  beforeEach(() => {
    search.value = { extension_origin: origin, code_challenge: challenge, state: 's1' }
  })
  afterEach(() => {
    cleanup()
    authorize.mockReset()
  })

  it('names the account and posts the code into this window on Allow', async () => {
    authorize.mockResolvedValue({ code: 'c1', state: 's1' })
    const post = vi.spyOn(window, 'postMessage')
    renderPage()
    expect(screen.getByText('sam@example.com')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Allow' }))
    expect(authorize).toHaveBeenCalledWith({
      extension_origin: origin,
      code_challenge: challenge,
      code_challenge_method: 'S256',
      state: 's1',
    })
    expect(post).toHaveBeenCalledWith(
      { type: 'linkdean-extension-code', code: 'c1', state: 's1' },
      window.location.origin
    )
    expect(await screen.findByText(/Connected/)).toBeTruthy()
    expect(JSON.parse(document.documentElement.dataset.linkdeanExtensionCode ?? '{}')).toMatchObject({ code: 'c1' })
    delete document.documentElement.dataset.linkdeanExtensionCode
    post.mockRestore()
  })

  it('shows why the server refused, and stays on the page', async () => {
    authorize.mockRejectedValue(new Error('this sign-in did not come from the Linkdean extension'))
    renderPage()
    await userEvent.click(screen.getByRole('button', { name: 'Allow' }))
    expect((await screen.findByRole('alert')).textContent).toContain('did not come from the Linkdean extension')
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
