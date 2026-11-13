import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest'
import type { UserSettings } from '@/schemas'
import { GeneralSettingsPanel } from './GeneralSettingsPanel'

vi.mock('@/hooks', () => ({
  useDashboards: () => ({ data: [] }),
  usePatchDashboard: () => ({ isPending: false, mutate: vi.fn() }),
  // No limits → QuotaUsagePanel renders nothing.
  useQuotaUsage: () => ({
    data: {
      limits: {},
      bookmarks: 0,
      stored_image_bytes: 0,
      enrich_actions_today: 0,
      enrich_period_start: '2026-01-01',
    },
    isLoading: false,
    isError: false,
  }),
}))

beforeAll(() => {
  // Mantine inputs measure themselves; jsdom has no ResizeObserver.
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe(): void {}
      unobserve(): void {}
      disconnect(): void {}
    }
  )
})

const settings: UserSettings = {
  user_id: 'u1',
  show_hidden: false,
  vault_search_mode: 'when_unlocked',
  theme: 'system',
  vault_auto_lock_minutes: 15,
  default_landing: '/',
  library_view: 'grid',
  trash_retention_days: 30,
  show_deleted_in_library: false,
  show_vault_in_library: true,
  vault_delete_mode: 'trash',
  show_keyboard_hints: true,
  updated_at: '2026-01-01T00:00:00Z',
}

describe('GeneralSettingsPanel', () => {
  afterEach(cleanup)

  it('saves the retention once, on blur, not per keystroke', async () => {
    const onPatch = vi.fn()
    render(
      <MantineProvider>
        <GeneralSettingsPanel settings={settings} saving={false} onPatch={onPatch} />
      </MantineProvider>
    )
    const input = screen.getByLabelText(/keep deleted items/i)
    await userEvent.clear(input)
    await userEvent.type(input, '365')
    expect(onPatch).not.toHaveBeenCalled()
    await userEvent.tab()
    expect(onPatch).toHaveBeenCalledTimes(1)
    expect(onPatch).toHaveBeenCalledWith({ trash_retention_days: 365 })
  })

  it('discards an empty retention instead of saving it', async () => {
    const onPatch = vi.fn()
    render(
      <MantineProvider>
        <GeneralSettingsPanel settings={settings} saving={false} onPatch={onPatch} />
      </MantineProvider>
    )
    const input = screen.getByLabelText(/keep deleted items/i)
    await userEvent.clear(input)
    await userEvent.tab()
    expect(onPatch).not.toHaveBeenCalled()
    expect((input as HTMLInputElement).value).toBe('30')
  })

  it('turns keyboard hints off', async () => {
    const onPatch = vi.fn()
    render(
      <MantineProvider>
        <GeneralSettingsPanel settings={settings} saving={false} onPatch={onPatch} />
      </MantineProvider>
    )
    const toggle = screen.getByRole('switch', { name: /show keyboard hints/i })
    expect((toggle as HTMLInputElement).checked).toBe(true)
    await userEvent.click(toggle)
    expect(onPatch).toHaveBeenCalledWith({ show_keyboard_hints: false })
  })
})
