import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { UpdateStatus } from '@/schemas'
import { SidebarVersion } from './SidebarVersion'

const state = vi.hoisted(() => ({
  version: '0.0.12' as string | undefined,
  status: { isSuccess: false, data: undefined } as { isSuccess: boolean; data?: UpdateStatus },
  enabled: [] as boolean[],
}))

vi.mock('@/hooks', () => ({
  useAppVersion: () => state.version,
  useUpdateStatus: (enabled: boolean) => {
    state.enabled.push(enabled)
    return state.status
  },
}))

function ready(update: UpdateStatus['update']): void {
  state.status = { isSuccess: true, data: { current: '0.0.12', update } }
}

function renderVersion(isAdmin: boolean, compact = false) {
  return render(
    <MantineProvider env="test">
      <SidebarVersion isAdmin={isAdmin} compact={compact} />
    </MantineProvider>
  )
}

afterEach(() => {
  cleanup()
  state.version = '0.0.12'
  state.status = { isSuccess: false, data: undefined }
  state.enabled = []
})

describe('SidebarVersion', () => {
  it('shows the version to everyone and asks about updates only for admins', () => {
    renderVersion(false)
    expect(screen.getByText('v0.0.12')).toBeInTheDocument()
    expect(state.enabled).toEqual([false])
  })

  it('renders nothing until the version is known', () => {
    state.version = undefined
    renderVersion(true)
    expect(screen.queryByText(/^v\d/)).toBeNull()
  })

  it.each([
    ['patch', 'v0.0.13 · Patch', /Safe to update/],
    ['minor', 'v0.1.0 · New features', /Safe to update/],
    ['major', 'v1.0.0 · Major', /Read the release notes/],
    ['breaking', 'v0.0.14 · Breaking', /breaking changes/],
  ] as const)('links a %s release to its notes', (level, label, hint) => {
    const latest = label.split(' ')[0].slice(1)
    ready({ latest, level, url: `https://github.com/arsamsarabi/linkdean/releases/tag/v${latest}` })
    renderVersion(true)
    const link = screen.getByRole('link', { name: hint })
    expect(link).toHaveTextContent(label)
    expect(link).toHaveAttribute('href', `https://github.com/arsamsarabi/linkdean/releases/tag/v${latest}`)
    expect(link).toHaveAttribute('target', '_blank')
  })

  it('shows no badge when the admin is up to date', () => {
    ready(null)
    renderVersion(true)
    expect(screen.getByText('v0.0.12')).toBeInTheDocument()
    expect(screen.queryByRole('link')).toBeNull()
  })

  it('keeps the rail compact: the version links to the notes', () => {
    ready({ latest: '0.0.13', level: 'patch', url: 'https://example.test/notes' })
    renderVersion(true, true)
    expect(screen.getByRole('link', { name: /v0\.0\.12\. Linkdean v0\.0\.13 is out/ })).toHaveTextContent('v0.0.12')
  })
})
