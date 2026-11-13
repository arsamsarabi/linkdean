import { useState, type ReactElement } from 'react'
import { Drawer, MantineProvider, TextInput } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useDrawerClosesOnEscape } from '@/hooks/render'
import { useDiscardGuard } from './useDiscardGuard'

function FormDrawer({ onClose }: { onClose: () => void }): ReactElement {
  const [name, setName] = useState('')
  const closeOnEscape = useDrawerClosesOnEscape()
  const { requestClose, discardModal } = useDiscardGuard(name !== '', onClose)
  return (
    <>
      <Drawer opened onClose={requestClose} closeOnEscape={closeOnEscape} title="Form">
        <TextInput label="Name" value={name} onChange={(e) => setName(e.currentTarget.value)} />
      </Drawer>
      {discardModal}
    </>
  )
}

function renderDrawer(): ReturnType<typeof vi.fn> {
  const onClose = vi.fn()
  render(
    <MantineProvider env="test">
      <FormDrawer onClose={onClose} />
    </MantineProvider>
  )
  return onClose
}

describe('useDiscardGuard', () => {
  afterEach(cleanup)

  it('closes an unchanged form at once', async () => {
    const onClose = renderDrawer()
    await userEvent.keyboard('{Escape}')
    expect(onClose).toHaveBeenCalledTimes(1)
    expect(screen.queryByText('Discard changes?')).toBeNull()
  })

  it('asks before closing a changed form, and Keep editing keeps it open', async () => {
    const onClose = renderDrawer()
    await userEvent.type(screen.getByLabelText('Name'), 'Rust')
    await userEvent.keyboard('{Escape}')
    expect(await screen.findByText('Discard changes?')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Keep editing' }))
    expect(onClose).not.toHaveBeenCalled()
    expect((screen.getByLabelText('Name') as HTMLInputElement).value).toBe('Rust')
  })

  it('closes after Discard', async () => {
    const onClose = renderDrawer()
    await userEvent.type(screen.getByLabelText('Name'), 'Rust')
    await userEvent.keyboard('{Escape}')
    await userEvent.click(await screen.findByRole('button', { name: 'Discard' }))
    expect(onClose).toHaveBeenCalledTimes(1)
  })

  it('Escape on the question keeps editing instead of closing the drawer', async () => {
    const onClose = renderDrawer()
    await userEvent.type(screen.getByLabelText('Name'), 'Rust')
    await userEvent.keyboard('{Escape}')
    await screen.findByText('Discard changes?')
    await userEvent.keyboard('{Escape}')
    expect(onClose).not.toHaveBeenCalled()
    expect(screen.getByLabelText('Name')).toBeTruthy()
  })
})
