import { useState, type ReactElement } from 'react'
import { Button, Drawer, MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ConfirmModal } from '@/components/confirm-modal/ConfirmModal'
import { useDrawerClosesOnEscape } from './useDialogLayers'

function DrawerWithConfirm({ onDrawerClose }: { onDrawerClose: () => void }): ReactElement {
  const closeOnEscape = useDrawerClosesOnEscape()
  const [confirm, setConfirm] = useState(false)
  return (
    <Drawer
      opened
      onClose={onDrawerClose}
      closeOnEscape={closeOnEscape}
      title="Drawer"
      transitionProps={{ duration: 0 }}
    >
      <Button onClick={() => setConfirm(true)}>Ask</Button>
      <ConfirmModal
        opened={confirm}
        onClose={() => setConfirm(false)}
        onConfirm={() => setConfirm(false)}
        title="Sure?"
        body="Really."
      />
    </Drawer>
  )
}

describe('dialog layers', () => {
  afterEach(cleanup)

  it('Esc closes only the dialog on top, then the drawer', async () => {
    const onDrawerClose = vi.fn()
    const user = userEvent.setup()
    render(
      <MantineProvider>
        <DrawerWithConfirm onDrawerClose={onDrawerClose} />
      </MantineProvider>
    )
    await user.click(screen.getByRole('button', { name: 'Ask' }))
    expect(await screen.findByText('Really.')).toBeTruthy()

    await user.keyboard('{Escape}')
    expect(onDrawerClose).not.toHaveBeenCalled()

    await user.keyboard('{Escape}')
    expect(onDrawerClose).toHaveBeenCalledTimes(1)
  })
})
