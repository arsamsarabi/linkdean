import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { TERMS_VERSION } from '@/lib'
import { AcceptableUsePage } from './AcceptableUsePage'
import { PrivacyPage } from './PrivacyPage'
import { TermsPage } from './TermsPage'

describe('legal pages', () => {
  afterEach(cleanup)

  it.each([
    ['Terms of Service', TermsPage],
    ['Privacy Policy', PrivacyPage],
    ['Acceptable Use Policy', AcceptableUsePage],
  ])('%s renders its title and the terms version', (title, Page) => {
    render(
      <MantineProvider>
        <Page />
      </MantineProvider>
    )
    expect(screen.getByRole('heading', { level: 1, name: title })).toBeTruthy()
    expect(screen.getByText(new RegExp(`Version ${TERMS_VERSION}`))).toBeTruthy()
  })
})
