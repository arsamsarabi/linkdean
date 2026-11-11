import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import type { ImportPreview } from '@/schemas'
import { ImportDuplicatesDrawer } from './ImportDuplicatesDrawer'

function dup(index: number) {
  return {
    index,
    url: `https://d${index}.example`,
    existing: {
      id: `b${index}`,
      title: `Mine ${index}`,
      description: null,
      tags: [],
      folders: ['Work'],
      updated_at: '2026-01-01T00:00:00Z',
    },
    incoming: { title: `Theirs ${index}`, description: null, tags: ['t'], folders: [] },
  }
}

const preview: ImportPreview = { total: 10, new: 8, duplicates: [dup(1), dup(4)], folders: { reused: 0, created: 0 } }

beforeAll(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})
afterEach(cleanup)

function renderDrawer(onConfirm = vi.fn(), onClose = vi.fn()) {
  render(
    <MantineProvider env="test">
      <ImportDuplicatesDrawer
        opened
        preview={preview}
        pending={false}
        error={null}
        onConfirm={onConfirm}
        onClose={onClose}
      />
    </MantineProvider>
  )
  return { onConfirm, onClose }
}

describe('ImportDuplicatesDrawer', () => {
  it('says how many are already in the library and skips them by default', async () => {
    const { onConfirm } = renderDrawer()
    expect(screen.getByText('2 of 10 bookmarks are already in your library.')).toBeTruthy()
    await userEvent.click(screen.getByRole('button', { name: 'Import' }))
    expect(onConfirm).toHaveBeenCalledWith({ default: 'skip', overrides: {} })
  })

  it('keeps a hand-picked row when apply-to-all changes', async () => {
    const { onConfirm } = renderDrawer()
    const row = screen.getByRole('group', { name: 'https://d4.example' })
    await userEvent.click(within(row).getByText('Import anyway'))
    await userEvent.click(within(screen.getByRole('group', { name: 'Apply to all' })).getByText('Replace'))
    await userEvent.click(screen.getByRole('button', { name: 'Import' }))
    expect(onConfirm).toHaveBeenCalledWith({ default: 'replace', overrides: { 4: 'keep' } })
  })

  it('cancel imports nothing', async () => {
    const { onConfirm, onClose } = renderDrawer()
    await userEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(onClose).toHaveBeenCalled()
    expect(onConfirm).not.toHaveBeenCalled()
  })
})
