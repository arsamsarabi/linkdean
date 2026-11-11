import { MantineProvider } from '@mantine/core'
import { cleanup, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { PlatformSettings, ScreenshotRetryStatus } from '@/schemas'
import { AdminScreenshotRetryPanel } from './AdminScreenshotRetryPanel'

const state = vi.hoisted(() => ({
  status: null as ScreenshotRetryStatus | null,
  retry: vi.fn(),
  save: vi.fn(),
}))

vi.mock('@/hooks', () => ({
  useScreenshotRetryStatus: () => ({ isError: false, data: state.status, refetch: vi.fn() }),
  useRetryScreenshots: () => ({ isPending: false, mutate: state.retry }),
  useUpdatePlatformSettings: () => ({ isPending: false, mutate: state.save }),
}))
vi.mock('@mantine/notifications', () => ({ notifications: { show: vi.fn() } }))

const settings = {
  registration_open: true,
  api_keys_enabled: true,
  require_verified_email: false,
  screenshot_retry_cooldown_hours: 24,
  screenshot_auto_retry_days: 7,
  updated_at: '2026-01-01T00:00:00Z',
} as PlatformSettings

const ready: ScreenshotRetryStatus = {
  failed: 12,
  cooldown_hours: 24,
  last_run_at: null,
  next_available_at: null,
  running: false,
  browser_configured: true,
}

function renderPanel(): void {
  render(
    <MantineProvider>
      <AdminScreenshotRetryPanel settings={settings} />
    </MantineProvider>
  )
}

const button = (): HTMLButtonElement => screen.getByRole('button', { name: /Retry failed screenshots/ })

describe('AdminScreenshotRetryPanel', () => {
  beforeEach(() => {
    state.status = { ...ready }
    state.retry = vi.fn()
    state.save = vi.fn()
  })
  afterEach(cleanup)

  it('shows the count and retries', async () => {
    renderPanel()
    expect(screen.getByText('12 failed screenshots can be retried')).toBeTruthy()
    await userEvent.click(button())
    expect(state.retry).toHaveBeenCalled()
  })

  it('is disabled during the cooldown and says when it is available', () => {
    state.status = { ...ready, next_available_at: '2026-11-12T10:00:00Z' }
    renderPanel()
    expect(button().disabled).toBe(true)
    expect(screen.getByText(/^Available again/)).toBeTruthy()
  })

  it('is disabled with nothing to retry, while running, or without a browser', () => {
    for (const s of [
      { failed: 0 },
      { running: true },
      { browser_configured: false },
    ] satisfies Partial<ScreenshotRetryStatus>[]) {
      state.status = { ...ready, ...s }
      renderPanel()
      expect(button().disabled).toBe(true)
      cleanup()
    }
  })

  it('saves a cooldown in range and ignores one out of range', async () => {
    renderPanel()
    const input = screen.getByRole('textbox', { name: 'Retry cooldown' })
    await userEvent.clear(input)
    await userEvent.type(input, '48')
    await userEvent.tab()
    expect(state.save).toHaveBeenCalledWith({ screenshot_retry_cooldown_hours: 48 }, expect.anything())
    state.save.mockClear()
    await userEvent.clear(input)
    await userEvent.type(input, '0')
    await userEvent.tab()
    expect(state.save).not.toHaveBeenCalled()
  })
})
