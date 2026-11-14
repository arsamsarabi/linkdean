import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest'
import { ImportExportPanel } from './ImportExportPanel'

const importMut = { isPending: false, mutateAsync: vi.fn() }
const previewMut = { isPending: false, mutateAsync: vi.fn() }
const flags = { agent_skills: false }

vi.mock('@/hooks', async (orig) => ({
  ...(await orig<typeof import('@/hooks')>()),
  useImportBookmarks: () => importMut,
  useImportPreview: () => previewMut,
  useMe: () => ({ data: { vault_unlocked: false } }),
  useExperimentalFlags: () => flags,
  useExportBookmarks: () => ({
    exportBookmarks: vi.fn(),
    isExporting: false,
    exportError: null,
    clearExportError: vi.fn(),
  }),
}))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))
// The Agent Skill card has its own tests.
vi.mock('./AgentSkillCard', () => ({ AgentSkillCard: () => <p>Agent Skill card</p> }))

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
beforeEach(() => {
  importMut.mutateAsync.mockReset().mockResolvedValue({ total: 1, imported: 1, failed: 0, skipped: 0, replaced: 0 })
  previewMut.mutateAsync.mockReset()
  flags.agent_skills = false
})
afterEach(cleanup)

async function pickFileAndImport(): Promise<void> {
  const input = document.querySelector('input[type="file"]') as HTMLInputElement
  await userEvent.upload(input, new File(['<DL></DL>'], 'b.html', { type: 'text/html' }))
  await userEvent.click(screen.getByRole('button', { name: 'Import' }))
}

describe('ImportExportPanel import', () => {
  it('imports straight away when nothing is a duplicate', async () => {
    previewMut.mutateAsync.mockResolvedValue({ total: 1, new: 1, duplicates: [], folders: { reused: 0, created: 0 } })
    render(
      <MantineProvider env="test">
        <ImportExportPanel />
      </MantineProvider>
    )
    await pickFileAndImport()
    await waitFor(() =>
      expect(importMut.mutateAsync).toHaveBeenCalledWith({ format: 'netscape', content: '<DL></DL>' })
    )
    expect(screen.queryByText('Review duplicates')).toBeNull()
  })

  it('opens the review when there are duplicates, and imports with the choices', async () => {
    previewMut.mutateAsync.mockResolvedValue({
      total: 2,
      new: 1,
      duplicates: [
        {
          index: 0,
          url: 'https://d.example',
          existing: {
            id: 'b',
            title: 'Mine',
            description: null,
            tags: [],
            folders: [],
            updated_at: '2026-01-01T00:00:00Z',
          },
          incoming: { title: 'Theirs', description: null, tags: [], folders: [] },
        },
      ],
      folders: { reused: 0, created: 0 },
    })
    render(
      <MantineProvider env="test">
        <ImportExportPanel />
      </MantineProvider>
    )
    await pickFileAndImport()
    expect(await screen.findByText('Review duplicates')).toBeTruthy()
    expect(importMut.mutateAsync).not.toHaveBeenCalled()
    const buttons = screen.getAllByRole('button', { name: 'Import' })
    await userEvent.click(buttons[buttons.length - 1])
    await waitFor(() =>
      expect(importMut.mutateAsync).toHaveBeenCalledWith({
        format: 'netscape',
        content: '<DL></DL>',
        duplicates: { default: 'skip', overrides: {} },
      })
    )
  })
})

describe('ImportExportPanel Agent Skill card', () => {
  function renderPanel(): void {
    render(
      <MantineProvider env="test">
        <ImportExportPanel />
      </MantineProvider>
    )
  }

  it('is hidden while the experimental flag is off', () => {
    renderPanel()
    expect(screen.queryByText('Agent Skill card')).toBeNull()
  })

  it('shows when the server turns the flag on', () => {
    flags.agent_skills = true
    renderPanel()
    expect(screen.getByText('Agent Skill card')).toBeTruthy()
  })
})
