import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { fakeBrowser } from 'wxt/testing/fake-browser'
import { savedIconItem, savedIconOfferedItem } from '@/lib/savedPages'
import { SavedIconOffer } from './SavedIconOffer'

function renderOffer(): void {
  render(
    <MantineProvider env="test">
      <SavedIconOffer />
    </MantineProvider>
  )
}

describe('SavedIconOffer', () => {
  beforeEach(() => {
    fakeBrowser.reset()
    // Mantine reads the colour scheme; jsdom has no matchMedia.
    vi.stubGlobal(
      'matchMedia',
      vi.fn().mockReturnValue({ matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() })
    )
    vi.spyOn(fakeBrowser.permissions, 'contains').mockResolvedValue(false as never)
  })
  afterEach(() => {
    cleanup()
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it('asks once, and Allow requests the tabs permission and turns the icon on', async () => {
    const request = vi.spyOn(fakeBrowser.permissions, 'request').mockResolvedValue(true as never)
    renderOffer()
    await userEvent.click(await screen.findByRole('button', { name: 'Allow' }))
    expect(request).toHaveBeenCalledWith({ permissions: ['tabs'] })
    await waitFor(async () => expect(await savedIconItem.getValue()).toBe(true))
    expect(await savedIconOfferedItem.getValue()).toBe(true)
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
  })

  it('Not now remembers the answer without asking the browser', async () => {
    const request = vi.spyOn(fakeBrowser.permissions, 'request')
    renderOffer()
    await userEvent.click(await screen.findByRole('button', { name: 'Not now' }))
    expect(request).not.toHaveBeenCalled()
    expect(await savedIconOfferedItem.getValue()).toBe(true)
    expect(await savedIconItem.getValue()).toBe(false)
  })

  it('stays hidden once answered, or when the icon is already on', async () => {
    await savedIconOfferedItem.setValue(true)
    renderOffer()
    await new Promise((r) => setTimeout(r, 0))
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
    cleanup()

    await savedIconOfferedItem.setValue(false)
    await savedIconItem.setValue(true)
    vi.spyOn(fakeBrowser.permissions, 'contains').mockResolvedValue(true as never)
    renderOffer()
    await new Promise((r) => setTimeout(r, 0))
    expect(screen.queryByRole('button', { name: 'Allow' })).toBeNull()
  })
})
