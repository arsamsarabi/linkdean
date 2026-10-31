import type { ReactElement } from 'react'
import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { QueryErrorAlert } from './QueryErrorAlert'

function wrap(ui: ReactElement): ReactElement {
  return <MantineProvider>{ui}</MantineProvider>
}

describe('QueryErrorAlert', () => {
  it('shows error message and retries on click', async (): Promise<void> => {
    const onRetry = vi.fn()
    const user = userEvent.setup()
    render(wrap(<QueryErrorAlert error={new Error('network down')} onRetry={onRetry} title="Load failed" />))
    expect(screen.getByText('Load failed')).toBeInTheDocument()
    expect(screen.getByText('network down')).toBeInTheDocument()
    await user.click(screen.getByRole('button', { name: /retry/i }))
    expect(onRetry).toHaveBeenCalledTimes(1)
  })
})
