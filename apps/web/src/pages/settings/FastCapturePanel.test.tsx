import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { FastCapturePanel } from './FastCapturePanel'

describe('FastCapturePanel', () => {
  afterEach(cleanup)

  it('offers a draggable bookmarklet pointing at this instance', () => {
    render(
      <MantineProvider env="test">
        <FastCapturePanel />
      </MantineProvider>
    )
    const link = screen.getByRole('link', { name: /Save to Linkdean/ })
    const href = link.getAttribute('href') ?? ''
    expect(href.startsWith('javascript:')).toBe(true)
    expect(href).toContain(JSON.stringify(`${window.location.origin}/save?`))
    expect(link.getAttribute('draggable')).toBe('true')
    expect(screen.getByRole('link', { name: 'How to set it up' }).getAttribute('href')).toBe(
      'https://docs.linkdean.app/docs/guides/fast-capture#iphone-and-ipad-a-shortcut'
    )
  })

  it('links the extension to its Chrome Web Store listing', () => {
    render(
      <MantineProvider env="test">
        <FastCapturePanel />
      </MantineProvider>
    )
    expect(screen.getByRole('link', { name: 'Add to Chrome' }).getAttribute('href')).toBe(
      'https://chromewebstore.google.com/detail/ielbdodidimpcolkjhkjdaifpfibolhf'
    )
  })
})
