import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { TERMS_VERSION } from '@/lib'
import { PrivacyPage } from './PrivacyPage'
import { TermsPage } from './TermsPage'

describe('legal pages', () => {
  afterEach(cleanup)

  it.each([
    ['Terms of Service', TermsPage],
    ['Privacy Policy', PrivacyPage],
  ])('%s renders its title and the terms version', (title, Page) => {
    render(
      <MantineProvider>
        <Page />
      </MantineProvider>
    )
    expect(screen.getByRole('heading', { level: 1, name: title })).toBeTruthy()
    expect(screen.getByText(new RegExp(`Version: ${TERMS_VERSION}`))).toBeTruthy()
  })

  it('carries the acceptable use rules as section 6 of the Terms', () => {
    const { container } = render(
      <MantineProvider>
        <TermsPage />
      </MantineProvider>
    )
    const section = container.querySelector('#acceptable-use')
    expect(section?.querySelector('h2')?.textContent).toBe('6. Acceptable use')
    expect(screen.getByRole('heading', { level: 3, name: '6.1 Illegal or harmful content' })).toBeTruthy()
    expect(screen.getByRole('heading', { level: 3, name: '6.3 Enforcement' })).toBeTruthy()
  })
})
