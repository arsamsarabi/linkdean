import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { McpConnectCard } from './McpConnectCard'

describe('McpConnectCard', () => {
  afterEach(cleanup)

  it('shows /api/mcp URL', () => {
    render(
      <MantineProvider env="test">
        <McpConnectCard />
      </MantineProvider>
    )
    expect(screen.getByText(new RegExp(`${window.location.origin}/api/mcp`))).toBeTruthy()
  })

  it('links to create an API key', () => {
    render(
      <MantineProvider env="test">
        <McpConnectCard />
      </MantineProvider>
    )
    const link = screen.getByRole('link', { name: /create an API key/i })
    expect(link.getAttribute('href')).toBe('/settings?tab=api-keys')
  })

  it('mentions write tools when the key allows', () => {
    render(
      <MantineProvider env="test">
        <McpConnectCard />
      </MantineProvider>
    )
    expect(screen.getByText(/create, update or trash bookmarks/i)).toBeTruthy()
    expect(screen.getByText(/library write/i)).toBeTruthy()
  })
})
