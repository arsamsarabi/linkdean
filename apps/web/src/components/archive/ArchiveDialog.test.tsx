import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ArchiveDialog } from './ArchiveDialog'

const one = vi.hoisted(() => ({ isPending: false, mutateAsync: vi.fn().mockResolvedValue([]) }))
const bulk = vi.hoisted(() => ({ isPending: false, mutateAsync: vi.fn().mockResolvedValue({ ok: 2, failed: [] }) }))

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useMe: () => ({ data: { settings: { archive_formats: ['html', 'pdf'] } } }),
  useRequestArchive: () => one,
  useBulkArchive: () => bulk,
}))

function show(props: { ids: string[]; replaces?: string | null }): void {
  render(
    <MantineProvider>
      <ArchiveDialog opened onClose={() => undefined} {...props} />
    </MantineProvider>
  )
}

describe('ArchiveDialog', () => {
  afterEach(() => {
    cleanup()
    vi.clearAllMocks()
  })

  it('starts from the formats in Settings and archives one bookmark', async () => {
    show({ ids: ['b1'] })
    await userEvent.click(screen.getByRole('checkbox', { name: /PDF/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Archive' }))
    expect(one.mutateAsync).toHaveBeenCalledWith({ id: 'b1', formats: ['html'] })
    expect(screen.queryByText(/replaces the copy/)).toBeNull()
  })

  it('warns that an older copy will be replaced', () => {
    show({ ids: ['b1'], replaces: '2026-11-03T10:00:00Z' })
    expect(screen.getByText(/This replaces the copy from 3 Nov 2026/)).toBeTruthy()
  })

  it('archives a selection in one request, and needs a format', async () => {
    show({ ids: ['a', 'b'] })
    await userEvent.click(screen.getByRole('checkbox', { name: /HTML/ }))
    await userEvent.click(screen.getByRole('checkbox', { name: /PDF/ }))
    expect(screen.getByRole('button', { name: 'Archive' }).hasAttribute('disabled')).toBe(true)
    await userEvent.click(screen.getByRole('checkbox', { name: /PDF/ }))
    await userEvent.click(screen.getByRole('button', { name: 'Archive' }))
    expect(bulk.mutateAsync).toHaveBeenCalledWith({ ids: ['a', 'b'], formats: ['pdf'] })
  })
})
