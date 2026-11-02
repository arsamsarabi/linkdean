import { MantineProvider } from '@mantine/core'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { FolderTile } from './FolderTile'

afterEach(cleanup)

describe('FolderTile', () => {
  it('keeps the menu button outside the tile button', () => {
    const onOpen = vi.fn()
    render(
      <MantineProvider>
        <FolderTile
          id="c1"
          name="Work"
          countLabel="3 items"
          onOpen={onOpen}
          access="owner"
          menu={{ onEdit: vi.fn() }}
        />
      </MantineProvider>
    )

    const tile = screen.getByRole('button', { name: 'Open folder Work, 3 items' })
    const menu = screen.getByRole('button', { name: 'Actions for Work' })
    expect(tile.contains(menu)).toBe(false)
    expect(tile.querySelector('button')).toBeNull()

    fireEvent.click(menu)
    expect(onOpen).not.toHaveBeenCalled()
    fireEvent.click(tile)
    expect(onOpen).toHaveBeenCalledWith('c1')
  })
})
