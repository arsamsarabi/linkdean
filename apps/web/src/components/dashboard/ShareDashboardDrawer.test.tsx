import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ShareDashboardDrawer } from './ShareDashboardDrawer'

vi.stubGlobal(
  'ResizeObserver',
  class {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
)

vi.mock('@/hooks', async (orig) => {
  const mutation = { mutateAsync: vi.fn(), isPending: false }
  return {
    ...(await orig<typeof import('@/hooks')>()),
    useDashboardShares: () => ({ data: { shares: [], pending: [] }, isLoading: false, isError: false }),
    useCreateDashboardShare: () => mutation,
    useUpdateDashboardShare: () => mutation,
    useDeleteDashboardShare: () => mutation,
    useUpdateDashboardPendingShare: () => mutation,
    useDeleteDashboardPendingShare: () => mutation,
  }
})

function renderDrawer(selfFillingSections: string[]): void {
  render(
    <MantineProvider env="test">
      <ShareDashboardDrawer
        dashboardId="d1"
        dashboardTitle="Home"
        opened
        onClose={() => undefined}
        selfFillingSections={selfFillingSections}
      />
    </MantineProvider>
  )
}

describe('ShareDashboardDrawer', () => {
  afterEach(cleanup)

  it('blocks sharing and lists the sections that fill themselves', () => {
    renderDrawer(['Latest', 'Recently visited'])
    expect(screen.getByText('Remove sections that fill themselves first')).toBeTruthy()
    expect(screen.getByText('Recently visited')).toBeTruthy()
    expect(screen.queryByRole('textbox', { name: /email/i })).toBeNull()
  })

  it('offers the invite form otherwise', () => {
    renderDrawer([])
    expect(screen.queryByText('Remove sections that fill themselves first')).toBeNull()
    expect(screen.getByRole('textbox', { name: /email/i })).toBeTruthy()
  })
})
