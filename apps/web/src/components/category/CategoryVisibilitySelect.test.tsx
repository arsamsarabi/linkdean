import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { CategoryVisibilitySelect } from './CategoryVisibilitySelect'

// jsdom has no ResizeObserver; Mantine's Select needs one.
vi.stubGlobal(
  'ResizeObserver',
  class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
)

function renderSelect(isVault: boolean): void {
  render(
    <MantineProvider>
      <CategoryVisibilitySelect value="private" isVault={isVault} onChange={vi.fn()} />
    </MantineProvider>
  )
}

describe('CategoryVisibilitySelect', () => {
  afterEach(cleanup)

  it('is disabled for vault categories', () => {
    renderSelect(true)
    const input = screen.getAllByLabelText('Who can see this').find((el) => el.tagName === 'INPUT')
    expect((input as HTMLInputElement).disabled).toBe(true)
  })

  it('explains that Members is not a Public link', () => {
    renderSelect(false)
    expect(screen.getByText(/use a public link for people without an account/i)).toBeTruthy()
  })
})
