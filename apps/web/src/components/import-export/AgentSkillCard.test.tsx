import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import { AgentSkillCard } from './AgentSkillCard'

const download = vi.fn()

vi.mock('@/hooks', () => ({
  useCategories: () => ({
    data: [
      { id: 'c1', name: 'Rust', parent_id: null, is_vault: false, deleted_at: null },
      { id: 'c2', name: 'Web', parent_id: 'c1', is_vault: false, deleted_at: null },
      { id: 'v1', name: 'Secrets', parent_id: null, is_vault: true, deleted_at: null },
    ],
  }),
  useTagNames: () => ({ data: ['design', 'rust'] }),
  useDownloadAgentSkill: () => ({ download, pending: false }),
}))

beforeAll(() => {
  // Mantine Select measures itself; jsdom has no ResizeObserver.
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})

describe('AgentSkillCard', () => {
  afterEach(() => {
    cleanup()
    download.mockReset()
  })

  it('downloads a picked folder, and never lists vault folders', async () => {
    render(
      <MantineProvider env="test">
        <AgentSkillCard />
      </MantineProvider>
    )
    const button = screen.getByRole('button', { name: /download skill/i })
    expect((button as HTMLButtonElement).disabled).toBe(true)
    await userEvent.click(screen.getByPlaceholderText('Pick a folder'))
    expect(screen.queryByRole('option', { name: 'Secrets' })).toBeNull()
    await userEvent.click(await screen.findByRole('option', { name: 'Rust / Web' }))
    await userEvent.click(button)
    expect(download).toHaveBeenCalledWith({ folder: 'c2' })
    expect(screen.getByText(/agent-skill\?folder=c2/)).toBeTruthy()
  })

  it('downloads a picked tag', async () => {
    render(
      <MantineProvider env="test">
        <AgentSkillCard />
      </MantineProvider>
    )
    await userEvent.click(screen.getByRole('radio', { name: 'Tag' }))
    await userEvent.click(screen.getByPlaceholderText('Pick a tag'))
    await userEvent.click(await screen.findByRole('option', { name: 'design' }))
    await userEvent.click(screen.getByRole('button', { name: /download skill/i }))
    expect(download).toHaveBeenCalledWith({ tag: 'design' })
  })
})
