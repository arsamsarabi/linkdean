import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { RecoveryKeyModal } from './RecoveryKeyModal'

describe('RecoveryKeyModal', () => {
  afterEach(cleanup)

  it('needs confirmation before closing', async () => {
    const onDone = vi.fn()
    render(
      <MantineProvider>
        <RecoveryKeyModal recoveryKey="ABCD-EFGH" onDone={onDone} />
      </MantineProvider>
    )
    expect(screen.getByText('ABCD-EFGH')).toBeTruthy()
    const done = screen.getByRole('button', { name: 'Done' })
    expect(done.hasAttribute('disabled') || done.getAttribute('data-disabled') === 'true').toBe(true)
    await userEvent.click(screen.getByLabelText('I saved this recovery key'))
    await userEvent.click(screen.getByRole('button', { name: 'Done' }))
    expect(onDone).toHaveBeenCalled()
  })
})
