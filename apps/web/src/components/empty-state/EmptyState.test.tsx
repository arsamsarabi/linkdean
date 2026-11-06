import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { EmptyState } from './EmptyState'

afterEach(cleanup)

describe('EmptyState', () => {
  it('renders a decorative illustration instead of an icon', () => {
    render(
      <MantineProvider env="test">
        <EmptyState illustration={{ src: '/mascot/dean-standing.webp' }} title="No bookmarks yet" />
      </MantineProvider>
    )
    const img = screen.getByAltText('')
    expect(img.getAttribute('src')).toBe('/mascot/dean-standing.webp')
  })
})
