import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import { BulkEditDrawer } from './BulkEditDrawer'

beforeAll(() => {
  // SegmentedControl measures itself; jsdom has no ResizeObserver.
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})

function renderTagDrawer(onApplyTag = vi.fn()) {
  render(
    <MantineProvider>
      <BulkEditDrawer
        opened
        onClose={vi.fn()}
        mode="tag"
        count={3}
        categories={[]}
        onApplyCategories={vi.fn()}
        onApplyTag={onApplyTag}
      />
    </MantineProvider>
  )
  return onApplyTag
}

describe('BulkEditDrawer tags', () => {
  afterEach(cleanup)

  it('will not replace tags with nothing', async () => {
    renderTagDrawer()
    await userEvent.click(screen.getByText('Replace tags'))
    expect(screen.getByRole('button', { name: 'Apply tags' }).hasAttribute('disabled')).toBe(true)
  })

  it('asks before replacing tags', async () => {
    const onApplyTag = renderTagDrawer()
    await userEvent.click(screen.getByText('Replace tags'))
    const input = screen.getAllByLabelText('Tags').find((el) => el.tagName === 'INPUT')!
    await userEvent.type(input, 'news{enter}')
    await userEvent.click(screen.getByRole('button', { name: 'Apply tags' }))
    expect(onApplyTag).not.toHaveBeenCalled()
    await userEvent.click(await screen.findByRole('button', { name: 'Replace tags' }))
    expect(onApplyTag).toHaveBeenCalledWith(['news'], 'replace')
  })
})
