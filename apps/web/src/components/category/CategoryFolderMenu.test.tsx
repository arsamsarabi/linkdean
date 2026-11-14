import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { CategoryFolderMenu } from './CategoryFolderMenu'

function renderMenu(isVault: boolean, onAgentSkill = vi.fn()) {
  render(
    <MantineProvider env="test">
      <CategoryFolderMenu name="Rust" access="read" isVault={isVault} onAgentSkill={onAgentSkill} onFork={vi.fn()} />
    </MantineProvider>
  )
  return onAgentSkill
}

describe('CategoryFolderMenu agent skill', () => {
  afterEach(cleanup)

  it('downloads a folder the viewer can read as an Agent Skill', async () => {
    const onAgentSkill = renderMenu(false)
    await userEvent.click(screen.getByRole('button', { name: 'Actions for Rust' }))
    await userEvent.click(await screen.findByRole('menuitem', { name: /agent skill/i }))
    expect(onAgentSkill).toHaveBeenCalledOnce()
  })

  it('never offers it for a vault folder', () => {
    renderMenu(true)
    expect(screen.queryByRole('button', { name: 'Actions for Rust' })).toBeNull()
  })
})
